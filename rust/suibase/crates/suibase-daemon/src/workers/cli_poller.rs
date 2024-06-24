// Child task of admin_controller
//
// One instance per workdir.
//
// Responsible to:
//  - Periodically and on-demand do "status" CLI commands and update globals.
//
// The task is auto-restart in case of panic.
//
// Design:
//    - Define a PollingTraitObject that does the "specialize" polling.
//    - Uses a PollerWorker for most of background task/event re-useable logic.
//

use crate::{
    admin_controller::AdminController,
    api::{StatusService, Versioned, WorkdirStatusResponse},
    shared_types::{Globals, WORKDIRS_KEYS},
};

use axum::async_trait;
use common::{
    basic_types::{AdminControllerTx, GenericTx, Instantiable, WorkdirContext, WorkdirIdx},
    workers::PollerWorker,
};

use common::workers::PollingTrait;

use tokio_graceful_shutdown::SubsystemHandle;

#[derive(Clone)]
pub struct CliPollerParams {
    globals: Globals,
    admctrl_tx: AdminControllerTx, // For exec shell messages
    workdir_idx: WorkdirIdx,
}

impl WorkdirContext for CliPollerParams {
    fn workdir_idx(&self) -> WorkdirIdx {
        self.workdir_idx
    }
}

impl CliPollerParams {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx, workdir_idx: WorkdirIdx) -> Self {
        Self {
            globals,
            admctrl_tx,
            workdir_idx,
        }
    }
}

pub struct CliPoller {
    // "Glue" the specialized PollingTraitObject with its parameters.
    // The worker does all the background task/events handling.
    poller: PollerWorker<PollingTraitObject, CliPollerParams>,
}

pub struct PollingTraitObject {
    params: CliPollerParams,
}

#[async_trait]
impl PollingTrait for PollingTraitObject {
    // This is called by the PollerWorker task.
    async fn update(&mut self) {
        self.update_globals_workdir_status().await;
    }
}

// This allow the PollerWorker to instantiate the PollingTraitObject.
impl Instantiable<CliPollerParams> for PollingTraitObject {
    fn new(params: CliPollerParams) -> Self {
        Self { params }
    }
}

impl CliPoller {
    pub fn new(params: CliPollerParams, subsys: &SubsystemHandle) -> Self {
        let poller =
            PollerWorker::<PollingTraitObject, CliPollerParams>::new(params.clone(), subsys);
        Self { poller }
    }

    pub fn get_tx_channel(&self) -> GenericTx {
        self.poller.get_tx_channel()
    }
}

impl PollingTraitObject {
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

    async fn update_globals_workdir_status(&mut self) {
        let workdir_idx = self.params.workdir_idx;
        let workdir = WORKDIRS_KEYS[workdir_idx as usize].to_string();

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
