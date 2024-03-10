use axum::async_trait;

use anyhow::Result;

use jsonrpsee::core::RpcResult;

use common::basic_types::WorkdirIdx;
use common::shared_types::GlobalsWorkdirsST;

use crate::admin_controller::{AdminControllerMsg, AdminControllerTx, EVENT_SHELL_EXEC};
use crate::shared_types::Globals;

use super::{
    GeneralApiServer, Header, RpcInputError, RpcSuibaseError, StatusService, VersionsResponse,
    WorkdirStatusResponse,
};

use super::def_header::Versioned;

pub struct GeneralApiImpl {
    pub globals: Globals,
    pub admctrl_tx: AdminControllerTx,
}

impl GeneralApiImpl {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
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
        resp: &mut WorkdirStatusResponse,
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
                        // (but skip if next word is "services" which is present only for remote network workdirs).
                        if let Some(status) = words.next() {
                            if status != "services" {
                                resp.status = Some(status.to_string());
                            } else if let Some(status) = words.next() {
                                resp.status = Some(status.to_string());
                            }
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

    async fn update_globals_workdir_status(
        &self,
        workdir: String,
        workdir_idx: WorkdirIdx,
        last_api_call_timestamp: &mut tokio::time::Instant,
    ) -> Result<Header> {
        // Debounce excessive refresh request on short period of time.
        if last_api_call_timestamp.elapsed() < tokio::time::Duration::from_millis(50) {
            let globals_read_guard = self.globals.get_status(workdir_idx).read().await;
            let globals = &*globals_read_guard;

            if let Some(ui) = &globals.ui {
                return Ok(ui.get_data().header.clone());
            }
        };
        *last_api_call_timestamp = tokio::time::Instant::now();

        // Try to refresh the globals and return the latest UUID.
        let mut resp = WorkdirStatusResponse::new();
        resp.header.method = "getWorkdirStatus".to_string();
        resp.header.key = Some(workdir.clone());

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
            is_successful =
                self.convert_status_cmd_resp_to_status_response(cmd_resp, workdir, &mut resp);
        }

        if !is_successful {
            // Command was not successful, make 100% sure the status is DOWN.
            resp.status = Some("DOWN".to_string());
        }

        {
            // Get the globals for the target workdir_idx.
            let mut globals_read_guard = self.globals.get_status(workdir_idx).write().await;
            let globals = &mut *globals_read_guard;
            if let Some(ui) = &mut globals.ui {
                // Update globals.ui with resp if different. This will update the uuid_data accordingly.
                let uuids = ui.set(&resp);

                // Make the inner header in the response have the proper uuids.
                resp.header.set_from_uuids(&uuids);
            } else {
                // Initialize globals.ui with resp.
                let new_versioned_resp = Versioned::new(resp.clone());
                // Copy the newly created UUID in the inner response header (so the caller can use these also).
                new_versioned_resp.write_uuids_into_header_param(&mut resp.header);
                globals.ui = Some(new_versioned_resp);
            }
        }

        Ok(resp.header)
    }
}

#[async_trait]
impl GeneralApiServer for GeneralApiImpl {
    async fn get_versions(&self, workdir: String) -> RpcResult<VersionsResponse> {
        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match self.globals.get_workdir_idx_by_name(&workdir).await {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Initialize some of the header fields of the response.
        let mut resp = VersionsResponse::new();
        resp.header.method = "getVersions".to_string();
        resp.header.key = Some(workdir.clone());

        // Allow only one API request for a given workdir at the time to avoid race conditions.
        let mut api_mutex_guard = self.globals.get_api_mutex(workdir_idx).lock().await;
        let api_mutex = &mut *api_mutex_guard;

        let last_api_call_timestamp = &mut api_mutex.last_api_call_timestamp;

        // Use the internal implementation (same logic as done with get_versions).
        {
            let update_result = self
                .update_globals_workdir_status(workdir, workdir_idx, last_api_call_timestamp)
                .await;

            // Read access to globals for versioning all components.
            // If no change, then the version remains the same for that global component.
            if let Ok(header) = update_result {
                resp.versions.push(header);
            }
        }

        // Initialize the uuids in the response header.
        // Use api_mutex.last_responses to detect if this response is equivalent to the previous one.
        // If not, increment the uuid_data.
        let last = &mut api_mutex.last_responses;
        if let Some(last_versions) = &mut last.versions {
            // Update globals.ui with resp if different. This will update the uuid_data accordingly.
            let uuids = last_versions.set(&resp);
            // Make the inner header in the response have the proper uuids.
            resp.header.set_from_uuids(&uuids);
        } else {
            // First time, so initialize the versioning logic with the current response.
            let new_versioned_resp = Versioned::new(resp.clone());
            // Copy the newly created UUID in the inner response header (so the caller can use these also).
            new_versioned_resp.write_uuids_into_header_param(&mut resp.header);
            last.versions = Some(new_versioned_resp);
        }

        Ok(resp)
    }

    async fn get_workdir_status(
        &self,
        workdir: String,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<WorkdirStatusResponse> {
        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match self.globals.get_workdir_idx_by_name(&workdir).await {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        if method_uuid.is_none() && data_uuid.is_none() {
            // Best-effort refresh of the status, since user is requesting for the latest.

            // Allow only one API request for a given workdir at the time to avoid race conditions.
            let mut api_mutex_guard = self.globals.get_api_mutex(workdir_idx).lock().await;
            let api_mutex = &mut *api_mutex_guard;

            let last_api_call_timestamp = &mut api_mutex.last_api_call_timestamp;

            // Use the internal implementation (same logic as done with get_versions).

            let _ = self
                .update_globals_workdir_status(
                    workdir.clone(),
                    workdir_idx,
                    last_api_call_timestamp,
                )
                .await;
        }

        {
            let globals_read_guard = self.globals.get_status(workdir_idx).read().await;
            let globals = &*globals_read_guard;

            if let Some(ui) = &globals.ui {
                if method_uuid.is_some() || data_uuid.is_some() {
                    let mut are_same_version = false;
                    if let (Some(method_uuid), Some(data_uuid)) =
                        (method_uuid.as_ref(), data_uuid.as_ref())
                    {
                        let globals_data_uuid = &ui.get_uuid().get_data_uuid();
                        if data_uuid == globals_data_uuid {
                            let globals_method_uuid = &ui.get_uuid().get_method_uuid();
                            if method_uuid == globals_method_uuid {
                                are_same_version = true;
                            }
                        }
                    }
                    if !are_same_version {
                        // Something went wrong, but this could be normal if the globals just got updated
                        // and the caller is not yet aware of it (assume the caller will eventually discover
                        // the latest version with getVersions).
                        return Err(RpcSuibaseError::OutdatedUUID().into());
                    }
                }
                let resp = ui.get_data().clone();
                //ui.write_uuids_into_header_param(&mut resp.header);
                return Ok(resp);
            } else {
                return Err(
                    RpcSuibaseError::InternalError("globals.ui was None".to_string()).into(),
                );
            }
        }
    }
}
