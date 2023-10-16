use core::fmt;
use std::fmt::Display;
use tokio::sync::Mutex;

use axum::async_trait;

use anyhow::Result;

use futures::future::ok;
use jsonrpsee::core::RpcResult;

use crate::admin_controller::{
    AdminControllerMsg, AdminControllerTx, EVENT_NOTIF_CONFIG_FILE_CHANGE, EVENT_SHELL_EXEC,
};
use crate::basic_types::{TargetServerIdx, WorkdirIdx};
use crate::shared_types::{
    Globals, GlobalsProxyMT, GlobalsStatusMT, GlobalsWorkdirStatusST, GlobalsWorkdirsST,
    ServerStats, TargetServer, UuidST, Workdir,
};

use super::{
    ModuleConfig, ModulesApiServer, ModulesConfigResponse, RpcInputError, RpcServerError,
    SuccessResponse, SuiEventsResponse,
};

use super::def_header::{Header, Versioned};

pub struct ModulesApiImpl {
    pub globals: Globals,
    pub admctrl_tx: AdminControllerTx,
    // TODO Change this to be per workdir.
    config_mutex: Mutex<tokio::time::Instant>,
}

impl ModulesApiImpl {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
            config_mutex: Mutex::new(tokio::time::Instant::now()),
        }
    }
}

#[async_trait]
impl ModulesApiServer for ModulesApiImpl {
    async fn get_events(
        &self,
        workdir: String,
        _after_ts: Option<String>,
        _last_ts: Option<String>,
    ) -> RpcResult<SuiEventsResponse> {
        // data/display/debug allow variations of how the output
        // is produced (and they may be combined).
        //
        // They all default to false when not specified
        // with the exception of data defaulting to true when
        // the other (display and debug) are false.
        //

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let _workdir_idx =
            match GlobalsWorkdirsST::find_workdir_idx_by_name(&self.globals, &workdir).await {
                Some(workdir_idx) => workdir_idx,
                None => {
                    return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into())
                }
            };

        // Initialize some of the header fields of the response.
        let mut resp = SuiEventsResponse::new();
        resp.header.method = "getEvents".to_string();
        resp.header.key = Some(workdir.clone());
        Ok(resp)
    }

    async fn publish(
        &self,
        workdir: String,
        module_name: String,
        module_id: String,
    ) -> RpcResult<SuccessResponse> {
        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::find_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Insert the data in the globals.
        {
            let mut globals_write_guard = self.globals.modules_config.write().await;
            let globals = &mut *globals_write_guard;
            let globals = globals.workdirs.get_mut(workdir_idx);

            if globals.ui.is_none() {
                globals.ui = Some(Versioned::new(ModulesConfigResponse::new()));
            }
            let ui = globals.ui.as_mut().unwrap();

            let resp = ui.get_mut_data();

            if resp.modules.is_none() {
                let new_modules_vec = Vec::new();
                resp.modules = Some(new_modules_vec);
            }
            let modules = resp.modules.as_mut().unwrap();

            // if module_name is already in modules, find and update it with the module_id
            // else add module_name/module_id to modules.
            let mut found = false;
            for module in modules.iter_mut() {
                if module.name == Some(module_name.clone()) {
                    module.id = Some(module_id.clone());
                    found = true;
                    break;
                }
            }
            if !found {
                modules.push(ModuleConfig {
                    name: Some(module_name),
                    id: Some(module_id),
                });
            }
        }

        // Return success.
        let mut resp = SuccessResponse::new();
        resp.header.method = "publish".to_string();
        resp.header.key = Some(workdir.clone());
        resp.success = true;
        Ok(resp)
    }

    async fn get_modules_config(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<ModulesConfigResponse> {
        // data/display/debug allow variations of how the output
        // is produced (and they may be combined).
        //
        // They all default to false when not specified
        // with the exception of data defaulting to true when
        // the other (display and debug) are false.

        // TODO Implement display/debug requests.
        let debug = debug.unwrap_or(false);
        let display = display.unwrap_or(debug);
        let _data = data.unwrap_or(!(debug || display));

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::find_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        let mut resp_ready: Option<ModulesConfigResponse> = None;

        // Just return what is already built in-memory, or empty.
        {
            // Get the globals for the target workdir_idx.
            let globals_read_guard = self.globals.modules_config.read().await;
            let globals = &*globals_read_guard;
            let globals = globals.workdirs.get_if_some(workdir_idx);

            if let Some(globals) = globals {
                if let Some(ui) = &globals.ui {
                    if let (Some(method_uuid), Some(data_uuid)) = (method_uuid, data_uuid) {
                        let (globals_method_uuid, globals_data_uuid) = ui.get_uuid().get();
                        let globals_data_uuid = globals_data_uuid.to_string();
                        if data_uuid == globals_data_uuid {
                            let globals_method_uuid = globals_method_uuid.to_string();
                            if method_uuid == globals_method_uuid {
                                // The caller requested the same data that it already have a copy of.
                                // Respond with the same UUID as a way to say "no change".
                                let mut resp = ModulesConfigResponse::new();
                                resp.header.method = "getModulesConfig".to_string();
                                resp.header.key = Some(workdir.clone());
                                ui.init_header_uuids(&mut resp.header);
                                resp_ready = Some(resp);
                            }
                        }
                    } else {
                        // The caller did not specify a method_uuid or data_uuid and
                        // there is an in-memory response ready. Just respond with it.
                        resp_ready = Some(ui.get_data().clone());
                    }
                }
            }
        }

        if resp_ready.is_none() {
            // No in-memory response available. Response will be empty (with no uuids).
            let mut resp = ModulesConfigResponse::new();
            resp.header.method = "getModulesConfig".to_string();
            resp.header.key = Some(workdir.clone());
            return Ok(resp);
        }

        Ok(resp_ready.unwrap())
    }
}
