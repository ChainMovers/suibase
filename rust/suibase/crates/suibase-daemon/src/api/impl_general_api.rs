use tokio::sync::Mutex;

use axum::async_trait;

use anyhow::Result;

use jsonrpsee::core::RpcResult;

use crate::admin_controller::{AdminControllerMsg, AdminControllerTx, EVENT_SHELL_EXEC};
use crate::basic_types::WorkdirIdx;
use crate::shared_types::{Globals, GlobalsWorkdirsST};

use super::{GeneralApiServer, RpcInputError, RpcSuibaseError, StatusResponse, StatusService};

use super::def_header::Versioned;

pub struct GeneralApiImpl {
    pub globals: Globals,
    pub admctrl_tx: AdminControllerTx,
    // TODO Change this to be per workdir.
    get_status_mutex: Mutex<tokio::time::Instant>,
}

impl GeneralApiImpl {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
            get_status_mutex: Mutex::new(tokio::time::Instant::now()),
        }
    }
    async fn shell_exec(&self, workdir_idx: WorkdirIdx, cmd: String) -> Result<String> {
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_SHELL_EXEC;
        let (tx, rx) = tokio::sync::oneshot::channel();
        msg.resp_channel = Some(tx);
        msg.workdir_idx = Some(workdir_idx);
        msg.data_string = Some(cmd);
        if (self.admctrl_tx.send(msg).await).is_ok() {
            match rx.await {
                Ok(resp_str) => {
                    return Ok(resp_str);
                }
                Err(e) => {
                    return Err(RpcSuibaseError::InternalError(e.to_string()).into());
                }
            }
        }
        Err(RpcSuibaseError::InternalError("admctrl_tx.send failed".to_string()).into())
    }

    fn remove_ascii_color_code(s: &str) -> String {
        let mut result = String::new();
        let mut is_color_code = false;
        for c in s.chars() {
            if is_color_code {
                if c == 'm' {
                    is_color_code = false;
                }
            } else if c == '\x1b' {
                is_color_code = true;
            } else {
                result.push(c);
            }
        }
        result
    }

    fn convert_status_cmd_resp_to_status_response(
        &self,
        cmd: String,
        workdir_name: String,
        resp: &mut StatusResponse,
    ) -> bool {
        // First line is two words, first should match the workdir name followed by the status word.
        // If the workdir name does not match, then the resp.status is set to "DOWN" else the status word is stores in resp.status.

        // Iterate every lines of cmd.
        let mut line_number = 0;

        let cmd = Self::remove_ascii_color_code(&cmd);
        for line in cmd.lines() {
            // Ignore lines starting with a "---" divider.
            if line.trim_start().starts_with("---") {
                continue;
            }

            line_number += 1;

            // Split the line into words.
            let mut words = line.split_whitespace();

            if line_number == 1 {
                // Get the very first word.
                if let Some(word) = words.next() {
                    if word == workdir_name {
                        // The first word matches the workdir name, so the next word is the status.
                        if let Some(status) = words.next() {
                            resp.status = Some(status.to_string());
                        }
                    } else {
                        resp.status = Some("DOWN".to_string());
                        resp.status_info = Some(format!(
                            "Missing status in [{}] first word is [{}]",
                            cmd, word
                        ));
                        return false;
                    }
                }

                if resp.status.is_none() {
                    // Something is not right.
                    resp.status = Some("DOWN".to_string());
                    resp.status_info = Some(format!("Missing status in {}", cmd));
                    return false;
                }

                continue; // Done with parsing first line
            }
            // Use first word in words to decide how to parse the remaining words.
            let first_word = words.next();

            match first_word {
                Some("localnet") | Some("faucet") | Some("multi-link") | Some("proxy") => {
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
                        return false;
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
                    // TODO
                }
                Some("asui") => {
                    // Parse asui line.
                    // TODO
                }
                _ => {
                    // Unknown line, so ignore it.
                }
            }
        }

        true
    }
}

#[async_trait]
impl GeneralApiServer for GeneralApiImpl {
    async fn get_status(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<StatusResponse> {
        // data/display/debug allow variations of how the output
        // is produced (and they may be combined).
        //
        // They all default to false when not specified
        // with the exception of data defaulting to true when
        // the other (display and debug) are false.
        //
        let debug = debug.unwrap_or(false);
        let display = display.unwrap_or(debug);
        let data = data.unwrap_or(!(debug || display));

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::find_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Initialize some of the header fields of the response.
        let mut resp = StatusResponse::new();
        resp.header.method = "getStatus".to_string();
        resp.header.key = Some(workdir.clone());

        // TODO: Consider refactoring as follow:
        //         get_data_if_no_change(request_uuids,resp)
        //         set_data_when_changed(new_data: T,resp)

        // Check if GlobalsStatus need to be refresh, if not, then
        // just return what is already loaded in-memory.
        //let now = tokio::time::Instant::now();
        let mut resp_ready = false;
        let mut force_resp_init = true;
        {
            // Get the globals for the target workdir_idx.
            let globals_read_guard = self.globals.status.read().await;
            let globals = &*globals_read_guard;
            let globals = globals.workdirs.get_if_some(workdir_idx);

            if let Some(globals) = globals {
                if let Some(ui) = &globals.ui {
                    force_resp_init = false;
                    if globals.last_ui_update.elapsed() < tokio::time::Duration::from_millis(200) {
                        // There is no need for a refresh, so initialize the response now.
                        if data && !debug && !display {
                            // Optimization for when requesting only the JSON output.
                            // If no change since the specified user Uuids in the request, then
                            // return an empty response (just echo the Uuids).
                            if let (Some(method_uuid), Some(data_uuid)) = (method_uuid, data_uuid) {
                                let globals_data_uuid = ui.get_uuid().get_data_uuid();
                                if data_uuid == globals_data_uuid {
                                    let globals_method_uuid = ui.get_uuid().get_method_uuid();
                                    if method_uuid == globals_method_uuid {
                                        ui.init_header_uuids(&mut resp.header);
                                        resp_ready = true;
                                    }
                                }
                            }
                        }

                        if !resp_ready {
                            // Respond with the latest version in globals.
                            resp = ui.get_data().clone();
                            ui.init_header_uuids(&mut resp.header);
                            // Remove fields that are not requested.
                            if !display {
                                resp.display = None;
                            }
                            if !debug {
                                resp.debug = None;
                            }
                            resp_ready = true;
                        }
                    }
                }
            }
        }

        if resp_ready {
            return Ok(resp);
        }

        // If reaching here, then the globals may need to be refreshed.
        {
            // Allow only one API request at the time to modify the Status globals,
            // Debounce excessive refresh request on short period of time.
            let mut mutex_guard = self.get_status_mutex.lock().await;
            let last_refresh = &mut *mutex_guard;
            if force_resp_init || last_refresh.elapsed() >= tokio::time::Duration::from_millis(50) {
                // Get an update with a "<workdir> status --json" shell call.
                // Map it into the resp.
                let cmd_resp = match self
                    .shell_exec(workdir_idx, format!("{} status", workdir))
                    .await
                {
                    Ok(cmd_resp) => cmd_resp,
                    Err(e) => format!("Error: {e}"),
                };

                // Do not assumes that if shell_exec returns OK that the command was successful.
                // The command execution may have failed, but the shell_exec itself may have succeeded.
                // Suibase often includes "Error:" somewhere in the CLI output.

                // Check if a line starts with "Error:" in cmd_resp.
                let mut is_successful = cmd_resp
                    .lines()
                    .all(|line| !line.trim_start().starts_with("Error:"));

                if is_successful {
                    is_successful = self
                        .convert_status_cmd_resp_to_status_response(cmd_resp, workdir, &mut resp);
                }
                let is_successful = is_successful; // Make is_successful immutable.

                {
                    // Get the globals for the target workdir_idx.
                    let mut globals_read_guard = self.globals.status.write().await;
                    let globals = &mut *globals_read_guard;
                    let globals = globals.workdirs.get_mut(workdir_idx);

                    if !is_successful {
                        // Command was not successful, so return a DOWN status but initialize the rest to last known states
                        // (if available uses the valid data from globals).
                        if let Some(globals_ui) = &globals.ui {
                            resp = globals_ui.get_data().clone();
                        }
                        // Force the status DOWN.
                        // At this point, "resp.status_info" should be already set with something useful to debug.
                        resp.status = Some("DOWN".to_string());
                    }

                    // Update globals with resp if different, and update the Uuid accordingly.
                    // Also, initialize 'resp' with the same Uuids as stored in global.
                    if let Some(globals_ui) = &mut globals.ui {
                        if resp != *globals_ui.get_data() {
                            globals_ui.set(&resp);
                        }
                        globals_ui.init_header_uuids(&mut resp.header);
                    } else {
                        let new_versioned_resp = Versioned::new(resp.clone());
                        new_versioned_resp.init_header_uuids(&mut resp.header);
                        globals.ui = Some(new_versioned_resp);
                    }

                    // Update the timestamps right before releasing the locks.
                    let now = tokio::time::Instant::now();
                    globals.last_ui_update = now;
                    *last_refresh = now;
                }
            }
        }

        Ok(resp)
    }
}
