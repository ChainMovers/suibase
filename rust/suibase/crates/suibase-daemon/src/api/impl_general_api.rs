use axum::async_trait;

use jsonrpsee::core::RpcResult;

use crate::admin_controller::{AdminController, AdminControllerTx};
use crate::shared_types::{Globals, GlobalsWorkdirsST};

use super::{
    GeneralApiServer, Header, RpcInputError, RpcSuibaseError, SuccessResponse, VersionsResponse,
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

    fn convert_set_active_cmd_resp_to_success_response(
        cmd_response: String,
        workdir_name: String,
        resp: &mut SuccessResponse,
    ) -> bool {
        log::info!(
            "convert_set_active_cmd_resp_to_success_response: cmd_response=[{}]",
            cmd_response
        );

        // Iterate every lines of the cmd response until one is parsed correctly.
        //
        // After one is parsed correctly, keep iterating in case of multiple lines with
        // one showing an error.
        let mut success = false;

        let cmd = common::utils::remove_ascii_color_code(&cmd_response);
        for line in cmd.lines() {
            if line.trim_start().starts_with("Error:") {
                resp.result = false;
                return false;
            }

            // Ignore lines starting with a "---" divider.
            if line.trim_start().starts_with("---") {
                continue;
            }

            // Split the line into words.
            let mut words = line.split_whitespace();

            let mut parse_ok = true;
            // The first word should match the workdir name
            if let Some(word) = words.nth(0) {
                if word != workdir_name {
                    parse_ok = false;
                }
            } else {
                parse_ok = false;
            }

            if parse_ok {
                // The last word should be "active" if successfully/already active.
                if let Some(last_word) = words.nth_back(0) {
                    if last_word != "active" {
                        parse_ok = false;
                    }
                } else {
                    parse_ok = false;
                }
            }
            if parse_ok {
                // The before last word should be either "now" or "already".
                if let Some(before_last_word) = words.nth_back(0) {
                    if before_last_word != "now" && before_last_word != "already" {
                        parse_ok = false;
                    }
                } else {
                    parse_ok = false;
                }
            }

            if parse_ok {
                success = true;
            }
        }

        resp.result = success;
        success
    }
}

#[async_trait]
impl GeneralApiServer for GeneralApiImpl {
    async fn workdir_command(
        &self,
        workdir: String,
        command: String,
    ) -> RpcResult<SuccessResponse> {
        // Prevent shell injection by validating the workdir (and forcing to use it as first CLI arg).
        let workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Prevent shell injection by not allowing some bash ways to chain commands.
        if command.contains(';') || command.contains('&') || command.contains('|') {
            return Err(RpcInputError::InvalidParams("command".to_string(), command).into());
        }

        // TODO Whitelist here the workdir commands that are allowed to be executed.

        let mut resp = SuccessResponse::new();
        resp.header.method = "workdirCommand".to_string();
        resp.header.key = Some(workdir.clone());

        let mut api_mutex_guard = self.globals.get_api_mutex(workdir_idx).lock().await;
        let _api_mutex = &mut *api_mutex_guard;

        let cmd_resp = match AdminController::send_shell_exec(
            &self.admctrl_tx,
            workdir_idx,
            format!("{} {}", workdir, command),
        )
        .await
        {
            Ok(cmd_resp) => cmd_resp,
            Err(e) => format!("Error: {e}"),
        };

        // User *might* have changed a state of Suibase... update the status now (instead of waiting for next audit).
        let _ = AdminController::send_event_update(&self.admctrl_tx, workdir_idx).await;

        // Return the response to the caller... can't interpret if successful.
        resp.result = true;
        resp.info = Some(cmd_resp);

        Ok(resp)
    }

    async fn get_versions(&self, workdir: Option<String>) -> RpcResult<VersionsResponse> {
        // If workdir is not specified, then default to the active workdir (asui).
        let asui_selection = self.globals.get_asui_selection().await;
        let workdir = if workdir.is_some() {
            workdir
        } else {
            asui_selection
        };

        if workdir.is_none() {
            return Err(RpcSuibaseError::InfoError(
                "Backend initializing. Active directory not yet identified".to_string(),
            )
            .into());
        }
        let workdir = workdir.unwrap();

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Initialize some of the header fields of the response.
        let mut resp = VersionsResponse::new();
        resp.header.method = "getVersions".to_string();
        resp.header.key = Some(workdir.clone());
        resp.header.semver = Some(env!("CARGO_PKG_VERSION").to_string());

        // Section for getWorkdirStatus version.
        {
            let asui_selection = self.globals.get_asui_selection().await;
            let globals_read_guard = self.globals.get_status(workdir_idx).read().await;
            let globals = &*globals_read_guard;

            if let Some(ui) = &globals.ui {
                // Create an header that has the same UUID as the globals.
                let mut hdr = Header::new("getWorkdirStatus");
                hdr.set_from_uuids(ui.get_uuid());
                resp.versions.push(hdr);
                resp.asui_selection = asui_selection;
            } else {
                return Err(RpcSuibaseError::InfoError(
                    "Backend initializing. Status not yet retreived".to_string(),
                )
                .into());
            }
        }

        // Section for getWorkdirPackages version.
        {
            // Get the data from the globals.get_packages
            let globals_read_guard = self.globals.get_packages(workdir_idx).read().await;
            let globals = &*globals_read_guard;
            if let Some(ui) = &globals.ui {
                // Create an header that has the same UUID as the globals.
                let mut hdr = Header::new("getWorkdirPackages");
                hdr.set_from_uuids(ui.get_uuid());
                resp.versions.push(hdr);
            }
        }

        // Initialize the uuids in the response header.
        // Use api_mutex.last_responses to detect if this response is equivalent to the previous one.
        // If not, increment the uuid_data.
        {
            let mut api_mutex_guard = self.globals.get_api_mutex(workdir_idx).lock().await;
            let api_mutex = &mut *api_mutex_guard;

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
        let workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        {
            let globals_read_guard = self.globals.get_status(workdir_idx).read().await;
            let globals = &*globals_read_guard;

            if let Some(ui) = &globals.ui {
                if let (Some(method_uuid), Some(data_uuid)) = (method_uuid, data_uuid) {
                    let mut are_same_version = false;
                    let globals_data_uuid = ui.get_uuid().get_data_uuid();
                    if data_uuid == globals_data_uuid {
                        let globals_method_uuid = ui.get_uuid().get_method_uuid();
                        if method_uuid == globals_method_uuid {
                            are_same_version = true;
                        }
                    }

                    if !are_same_version {
                        // Something went wrong, but this could be normal if the globals just got updated
                        // and the caller is not yet aware of it (assume the caller will eventually discover
                        // the latest version with getVersions).
                        return Err(RpcSuibaseError::OutdatedUUID().into());
                    }
                }
                let mut resp = ui.get_data().clone();
                resp.header.set_from_uuids(ui.get_uuid());
                return Ok(resp);
            } else {
                return Err(RpcSuibaseError::InfoError(
                    "Backend still initializing. Status not yet known".to_string(),
                )
                .into());
            }
        }
    }

    async fn set_asui_selection(&self, workdir: String) -> RpcResult<SuccessResponse> {
        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };
        let mut resp = SuccessResponse::new();
        resp.header.method = "setAsuiSelection".to_string();
        resp.header.key = Some(workdir.clone());
        resp.result = false; // Will change to true if applied successfully.

        let mut api_mutex_guard = self.globals.get_api_mutex(workdir_idx).lock().await;
        let _api_mutex = &mut *api_mutex_guard;

        // Call into the shell to set the asui selection.
        let cmd_resp = match AdminController::send_shell_exec(
            &self.admctrl_tx,
            workdir_idx,
            format!("{} set-active", workdir),
        )
        .await
        {
            Ok(cmd_resp) => cmd_resp,
            Err(e) => {
                log::error!("Error: {e}");
                format!("Error: {e}")
            }
        };

        // Do not assumes that if shell_exec returns OK that the command was successful.
        // Parse the command response to figure out if really successful.
        let is_successful =
            Self::convert_set_active_cmd_resp_to_success_response(cmd_resp, workdir, &mut resp);

        if !is_successful {
            // Command was not successful, make 100% sure the result is negative.
            resp.result = false;
        } else {
            // User changed a state of Suibase... update the status now (instead of waiting for next audit).
            let _ = AdminController::send_event_update(&self.admctrl_tx, workdir_idx).await;
        }

        Ok(resp)
    }

    async fn workdir_refresh(&self, workdir: String) -> RpcResult<SuccessResponse> {
        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Update the status now (instead of waiting for next audit).
        let _ = AdminController::send_event_update(&self.admctrl_tx, workdir_idx).await;

        let mut resp = SuccessResponse::new();
        resp.header.method = "workdirRefresh".to_string();
        resp.header.key = Some(workdir.clone());
        resp.result = true;
        Ok(resp)
    }
}
