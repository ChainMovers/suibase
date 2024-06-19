// Child thread of admin_controller
//
// One instance per workdir.
//
// Responsible to:
//  - Periodically and on-demand do "status" CLI commands and update globals.
//
// The thread is auto-restart in case of panic.

use std::sync::Arc;

use crate::{
    admin_controller::{AdminController, AdminControllerTx},
    api::{StatusService, Versioned, WorkdirStatusResponse},
    shared_types::{Globals, WORKDIRS_KEYS},
};

use anyhow::Result;

use axum::async_trait;
use common::{
    basic_types::{
        self, AutoThread, GenericChannelMsg, GenericRx, GenericTx, Runnable, WorkdirIdx,
    },
    mpsc_q_check,
};

use tokio::sync::Mutex;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use common::basic_types::remove_generic_event_dups;

#[derive(Clone)]
pub struct CliPollerParams {
    globals: Globals,
    event_rx: Arc<Mutex<GenericRx>>, // To receive MSPC messages.
    event_tx: GenericTx,             // To send messages to self.
    admctrl_tx: AdminControllerTx,   // To send messages to parent
    workdir_idx: WorkdirIdx,
    workdir_name: String,
}

impl CliPollerParams {
    pub fn new(
        globals: Globals,
        event_rx: GenericRx,
        event_tx: GenericTx,
        admctrl_tx: AdminControllerTx,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            event_tx,
            admctrl_tx,
            workdir_idx,
            workdir_name: WORKDIRS_KEYS[workdir_idx as usize].to_string(),
        }
    }
}

pub struct CliPollerWorker {
    auto_thread: AutoThread<CliPollerWorkerTask, CliPollerParams>,
}

impl CliPollerWorker {
    pub fn new(params: CliPollerParams) -> Self {
        Self {
            auto_thread: AutoThread::new(format!("CliPollerWorker-{}", params.workdir_idx), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct CliPollerWorkerTask {
    task_name: String,
    params: CliPollerParams,
    last_update_timestamp: Option<tokio::time::Instant>,
}

#[async_trait]
impl Runnable<CliPollerParams> for CliPollerWorkerTask {
    fn new(task_name: String, params: CliPollerParams) -> Self {
        Self {
            task_name,
            params,
            last_update_timestamp: None,
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

impl CliPollerWorkerTask {
    async fn process_audit_msg(&mut self, msg: GenericChannelMsg) {
        // This function takes care of periodic operation synchronizing
        // between the CLI state and the globals.

        if msg.event_id != basic_types::EVENT_AUDIT {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
                return;
            }
        } else {
            log::error!("Missing workdir_idx {:?}", msg);
            return;
        }

        // Simply convert the periodic audit into an update, but do
        // not force it.
        let force = false;
        self.update_globals_workdir_status(force).await;
    }

    async fn process_update_msg(&mut self, msg: GenericChannelMsg) {
        // This function takes care of synching from Suibase CLI to the globals.

        // Make sure the event_id is EVENT_UPDATE.
        if msg.event_id != basic_types::EVENT_UPDATE {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
                return;
            }
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
            return;
        }

        let force = true;
        self.update_globals_workdir_status(force).await;
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // Take mutable ownership of the event_rx channel as long this thread is running.
        let event_rx = Arc::clone(&self.params.event_rx);
        let mut event_rx = event_rx.lock().await;

        // Remove duplicate of EVENT_AUDIT and EVENT_UPDATE in the event_rx queue.
        // (handle the case where the task was auto-restarted).
        remove_generic_event_dups(&mut event_rx, &self.params.event_tx);
        mpsc_q_check!(event_rx); // Just to help verify if the Q unexpectedly "accumulate".

        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = event_rx.recv().await {
                common::mpsc_q_check!(event_rx);
                match msg.event_id {
                    basic_types::EVENT_AUDIT => {
                        // Periodic processing.
                        self.process_audit_msg(msg).await;
                    }
                    basic_types::EVENT_UPDATE => {
                        // On-demand/reactive processing.
                        self.process_update_msg(msg).await;
                    }
                    _ => {
                        log::error!("Unexpected event_id {:?}", msg);
                    }
                }
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    fn convert_status_cmd_resp_to_status_response(
        cmd_response: String,
        workdir_name: String,
        resp: &mut WorkdirStatusResponse,
    ) -> (bool, Option<String>) {
        // First line is two words, first should match the workdir name followed by the status word.
        // If the workdir name does not match, then the resp.status is set to "DOWN" else the status word is stores in resp.status.
        let mut first_line_parsed = false;
        let mut asui_selection: Option<String> = None;

        // Iterate every lines of cmd.
        let mut line_number = 0;
        let mut error_detected = false;

        let cmd = common::utils::remove_ascii_color_code(&cmd_response);
        for line in cmd.lines() {
            let line = line.trim();
            // Ignore empty lines or "---" divider.
            if line.is_empty() || line.starts_with("---") {
                continue;
            }

            if line.starts_with("Error:") {
                error_detected = true;
            }

            line_number += 1;

            // Detect into the first two lines for a hint of a problem.
            if line_number <= 2 {
                let line_lc = line.to_lowercase();
                // Detect Suibase not installed.
                if line_lc.contains("not initialized")
                    || line_lc.contains("not found")
                    || line_lc.contains("no such")
                    || line_lc.contains("no command")
                {
                    resp.status = Some("DISABLED".to_string());
                    let status_info = format!("{0} not initialized. Do '{0} start'", workdir_name);
                    resp.status_info = Some(status_info);
                    return (false, None);
                }
            }

            if error_detected {
                if line_number == 2 {
                    // Error detected but not sure what the problem is.
                    resp.status = Some("DOWN".to_string());
                    resp.status_info = Some(format!("Error detected [{}]", cmd_response));
                    log::error!("Workdir status error detected [{}]", cmd_response);
                    return (false, None);
                }
                continue;
            }

            // Split the line into words.
            let mut words = line.split_whitespace();

            if line_number == 1 {
                // Get the very first word.
                if let Some(word) = words.next() {
                    if word == workdir_name {
                        // The first word matches the workdir name, so the next word is the status.
                        // (but skip if next word is "services" which is present only for remote network workdirs).
                        if let Some(status) = words.next() {
                            if status != "services" {
                                resp.status = Some(status.to_string());
                                first_line_parsed = true;
                            } else if let Some(status) = words.next() {
                                resp.status = Some(status.to_string());
                                first_line_parsed = true;
                            }
                        }
                    } else {
                        resp.status = Some("DOWN".to_string());
                        resp.status_info = Some(format!(
                            "Missing status in [{}] first word is [{}]",
                            cmd, word
                        ));
                        return (false, None);
                    }
                }

                if resp.status.is_none() {
                    // Something is not right.
                    resp.status = Some("DOWN".to_string());
                    resp.status_info = Some(format!("Missing status in {}", cmd));
                    return (false, None);
                }

                continue; // Done with parsing first line
            }
            // Use first word in words to decide how to parse the remaining words.
            let first_word = words.next();

            match first_word {
                Some("Localnet") | Some("Faucet") | Some("Multi-link") | Some("Proxy") => {
                    // Get the 4th word in words.
                    let mut service_status = words.nth(2).unwrap_or("").to_string();

                    // Validate if service_status is one of substring "OK", "DOWN", "DEGRADED" or "NOT RUNNING"
                    let status_is_valid = if service_status == "OK"
                        || service_status == "DOWN"
                        || service_status == "DEGRADED"
                    {
                        true
                    } else if service_status == "NOT" {
                        // Special case for two words "NOT RUNNING" status.
                        let mut ret_value = false;
                        if let Some(next_word) = words.next() {
                            if next_word == "RUNNING" {
                                ret_value = true;
                                service_status = "NOT RUNNING".to_string();
                            } else {
                                service_status = format!("NOT {}", next_word);
                            }
                        }
                        ret_value
                    } else {
                        false
                    };
                    let service_status = service_status; // Make service_status immutable.

                    if !status_is_valid {
                        // Something is not right.
                        resp.status = Some("DOWN".to_string());
                        resp.status_info = Some(format!(
                            "Missing [{}] service status in [{}] service_status=[{}]",
                            first_word.unwrap(),
                            cmd,
                            service_status,
                        ));
                        return (false, None);
                    }

                    // Valid service status found, make sure the response has the services array initialized.
                    if resp.services.is_none() {
                        resp.services = Some(Vec::new());
                    }

                    // service label is everything before the ":" on the line.
                    let service_label = line.split(':').next().unwrap_or("").trim().to_string();
                    if service_label.is_empty() {
                        continue;
                    }

                    // Lookup if the service is already in resp.services. If not then create it, else
                    // just ignore this line.
                    let services = resp.services.as_mut().unwrap();
                    let mut found = false;
                    for service in services.iter_mut() {
                        if service.label == service_label {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        // Add a new service to resp.services.
                        let mut new_service = StatusService::new(service_label);
                        new_service.status = Some(service_status);
                        services.push(new_service);
                    }
                }
                Some("client") => {
                    // Parse client line.
                    let mut sui_version = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    if !sui_version.is_empty() {
                        // Remove leading "sui " from sui_version.
                        sui_version = sui_version.trim_start_matches("sui ").to_string();
                        resp.client_version = Some(sui_version);
                    }
                }
                Some("asui") => {
                    // Parse asui selection line. Isolate what is between [] on that line
                    let mut asui_selection_candidate =
                        line.split('[').nth(1).unwrap_or("").trim().to_string();
                    if !asui_selection_candidate.is_empty() {
                        // Remove trailing "]" from asui_selection.
                        asui_selection_candidate =
                            asui_selection_candidate.trim_end_matches(']').to_string();
                        // Trim spaces
                        asui_selection_candidate = asui_selection_candidate.trim().to_string();
                        // Validate that it is one of the known workdir key.
                        if WORKDIRS_KEYS.contains(&asui_selection_candidate.as_str()) {
                            // All good.
                            asui_selection = Some(asui_selection_candidate);
                        }
                    }
                }
                _ => {
                    // Unknown line, so ignore it.
                }
            }
        }

        (first_line_parsed, asui_selection)
    }

    async fn update_globals_workdir_status(&mut self, force: bool) {
        if !force {
            // Debounce excessive refresh request on short period of time.
            if let Some(last_cli_call_timestamp) = self.last_update_timestamp {
                if last_cli_call_timestamp.elapsed() < tokio::time::Duration::from_millis(50) {
                    return;
                }
            };
        }
        self.last_update_timestamp = Some(tokio::time::Instant::now());

        let workdir = &self.params.workdir_name;
        let workdir_idx = self.params.workdir_idx;

        // Try to refresh the globals and return the latest UUID.
        let mut resp = WorkdirStatusResponse::new();
        resp.header.method = "getWorkdirStatus".to_string();
        resp.header.key = Some(workdir.clone());

        // Get an update with a "<workdir> status" shell call.
        // Map it into the resp.
        let cmd_resp = match AdminController::send_shell_exec(
            &self.params.admctrl_tx,
            workdir_idx,
            format!("{} status --daemoncall", workdir),
        )
        .await
        {
            Ok(cmd_resp) => cmd_resp,
            Err(e) => format!("Error: {e}"),
        };

        // Do not assumes that if shell_exec returns OK that the command was successful.
        // Parse the command response to figure out if really successful.
        resp.status = None;
        let (is_successful, asui_selection) =
            Self::convert_status_cmd_resp_to_status_response(cmd_resp, workdir.clone(), &mut resp);

        // Default to DOWN if could not identify the status.
        if resp.status.is_none() {
            resp.status = Some("DOWN".to_string());
        }

        if is_successful && asui_selection.is_some() {
            self.params.globals.set_asui_selection(asui_selection).await;
        }

        {
            // Update the globals with this potentially new response.
            let mut globals_write_guard = self.params.globals.get_status(workdir_idx).write().await;
            let globals = &mut *globals_write_guard;
            if let Some(ui) = &mut globals.ui {
                // Update globals.ui with resp if different. This will update the uuid_data accordingly.
                let _was_updated = ui.take_if_not_equal(resp.clone());
                //if was_updated {
                //log::info!("Workdir {} status updated {:?}", workdir, resp);
                //}

                // Make the inner header in the response have the proper uuids.
                // resp.header.set_from_uuids(&uuids);
            } else {
                // Initialize globals.ui with resp.
                let new_versioned_resp = Versioned::new(resp);
                // Copy the newly created UUID in the inner response header (so the caller can use these also).
                //new_versioned_resp.write_uuids_into_header_param(&mut resp.header);
                globals.ui = Some(new_versioned_resp);
            }
        }
    }
}
