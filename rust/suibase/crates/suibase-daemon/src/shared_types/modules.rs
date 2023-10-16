use crate::{
    api::{ModulesConfigResponse, SuiEvents, Versioned},
    basic_types::AutoSizeVec,
};

#[derive(Debug, Clone)]
pub struct ModulesWorkdirConfig {
    // Mostly store everything in the same struct
    // as the response of the GetEventsConfig API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<ModulesConfigResponse>>,
    pub last_ui_update: tokio::time::Instant,
}

impl ModulesWorkdirConfig {
    pub fn new() -> Self {
        Self {
            ui: None,
            last_ui_update: tokio::time::Instant::now(),
        }
    }
}

impl std::default::Default for ModulesWorkdirConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GlobalsModulesConfigST {
    // One per workdir, WorkdirIdx maintained by workdirs.
    pub workdirs: AutoSizeVec<ModulesWorkdirConfig>,
}

impl GlobalsModulesConfigST {
    pub fn new() -> Self {
        Self {
            workdirs: AutoSizeVec::new(),
        }
    }
}

impl Default for GlobalsModulesConfigST {
    fn default() -> Self {
        Self::new()
    }
}
