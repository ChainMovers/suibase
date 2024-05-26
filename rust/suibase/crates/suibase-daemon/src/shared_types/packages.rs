use crate::api::{Versioned, WorkdirPackagesResponse};

//use common::basic_types::{AutoSizeVec, WorkdirIdx};

#[derive(Debug, Clone)]
pub struct PackagesWorkdirConfig {
    // Mostly store everything in the same struct
    // as the response of the GetEventsConfig API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<WorkdirPackagesResponse>>,
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
pub struct GlobalsWorkdirPackagesST {
    // Mostly store everything in the same struct
    // as the response of the GetWorkdirPackages API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<WorkdirPackagesResponse>>,
}

impl GlobalsWorkdirPackagesST {
    pub fn new() -> Self {
        Self { ui: None }
    }

    pub fn init_empty_ui(&mut self, workdir: String) {
        // As needed, initialize globals.ui with resp.
        let mut empty_resp = WorkdirPackagesResponse::new();
        empty_resp.header.method = "getWorkdirPackages".to_string();
        empty_resp.header.key = Some(workdir);

        let new_versioned_resp = Versioned::new(empty_resp.clone());
        // Copy the newly created UUID in the inner response header (so the caller can use these also).
        new_versioned_resp.write_uuids_into_header_param(&mut empty_resp.header);
        self.ui = Some(new_versioned_resp);
    }

}

impl Default for GlobalsWorkdirPackagesST {
    fn default() -> Self {
        Self::new()
    }
}
