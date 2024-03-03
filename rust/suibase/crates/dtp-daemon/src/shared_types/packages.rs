use std::collections::HashMap;

use crate::api::{MoveConfig, PackagesConfigResponse, Versioned};

use common::basic_types::{AutoSizeVec, WorkdirIdx};
#[derive(Debug, Clone)]
pub struct PackagesWorkdirConfig {
    // Mostly store everything in the same struct
    // as the response of the GetEventsConfig API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<PackagesConfigResponse>>,
    pub last_ui_update: tokio::time::Instant,
}

impl PackagesWorkdirConfig {
    pub fn new() -> Self {
        Self {
            ui: None,
            last_ui_update: tokio::time::Instant::now(),
        }
    }
}

impl std::default::Default for PackagesWorkdirConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GlobalsPackagesConfigST {
    // One per workdir, WorkdirIdx maintained by workdirs.
    pub workdirs: AutoSizeVec<PackagesWorkdirConfig>,
}

impl GlobalsPackagesConfigST {
    pub fn new() -> Self {
        Self {
            workdirs: AutoSizeVec::new(),
        }
    }

    // Convenient access to the move_configs for a given workdir.

    pub fn get_move_configs(
        workdirs: &AutoSizeVec<PackagesWorkdirConfig>,
        workdir_idx: WorkdirIdx,
    ) -> Option<&HashMap<String, MoveConfig>> {
        // Caller must hold a read lock on workdirs.
        // Will return None if an object is missing while trying to reach the MoveConfig (should not happen).
        let config_resp = Self::get_config_resp(workdirs, workdir_idx)?;
        Some(config_resp.move_configs.as_ref().unwrap())
    }

    pub fn get_mut_move_configs(
        workdirs: &mut AutoSizeVec<PackagesWorkdirConfig>,
        workdir_idx: WorkdirIdx,
    ) -> &mut HashMap<String, MoveConfig> {
        // Caller must hold a write lock on workdirs.
        // Will create the move_configs if does not exists.
        let config_resp = Self::get_mut_config_resp(workdirs, workdir_idx);

        config_resp.move_configs.as_mut().unwrap()
    }

    #[allow(clippy::question_mark)]
    pub fn get_config_resp(
        workdirs: &AutoSizeVec<PackagesWorkdirConfig>,
        workdir_idx: WorkdirIdx,
    ) -> Option<&PackagesConfigResponse> {
        // Will return None if an object is missing while trying to reach the PackageConfigResponse (should not happen).
        let packages_workdir_config = workdirs.get_if_some(workdir_idx)?;
        let ui = packages_workdir_config.ui.as_ref()?;
        let config_resp = ui.get_data();

        if config_resp.move_configs.is_none() {
            return None;
        }

        Some(config_resp)
    }

    pub fn get_mut_config_resp(
        workdirs: &mut AutoSizeVec<PackagesWorkdirConfig>,
        workdir_idx: WorkdirIdx,
    ) -> &mut PackagesConfigResponse {
        let packages_workdir_config = workdirs.get_mut(workdir_idx);
        if packages_workdir_config.ui.is_none() {
            packages_workdir_config.ui = Some(Versioned::new(PackagesConfigResponse::new()));
        }
        let ui = packages_workdir_config.ui.as_mut().unwrap();
        let config_resp = ui.get_mut_data();

        if config_resp.move_configs.is_none() {
            config_resp.move_configs = Some(HashMap::new());
        }

        config_resp
    }
}

impl Default for GlobalsPackagesConfigST {
    fn default() -> Self {
        Self::new()
    }
}
