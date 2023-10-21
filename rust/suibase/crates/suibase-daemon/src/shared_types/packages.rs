use crate::{
    api::{PackagesConfigResponse, Versioned},
    basic_types::AutoSizeVec,
};

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
}

impl Default for GlobalsPackagesConfigST {
    fn default() -> Self {
        Self::new()
    }
}
