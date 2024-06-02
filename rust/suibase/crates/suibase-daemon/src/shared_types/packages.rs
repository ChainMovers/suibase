use crate::api::{Versioned, WorkdirPackagesResponse};
use std::hash::{Hash, Hasher};

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

#[derive(Debug, Clone, Default)]
pub struct PackagePath {
    // This is an example of full path corresponding to a PackagePath:
    //   workdir_path/published-data/demo/HPDM7J4PRD6OTJTGEMMPTFN2TM/1716670905815
    //
    // Where "demo" is a package_name and "HPDM7J4PRD6OTJTGEMMPTFN2TM" is a
    // package_uuid and "1716670905815" is a package_timestamp.
    package_name: String,
    package_uuid: String,
    package_timestamp: String,
}

impl PackagePath {
    pub fn new(package_name: String, package_uuid: String, package_timestamp: String) -> Self {
        Self {
            package_name,
            package_uuid,
            package_timestamp,
        }
    }

    pub fn get_path(&self, published_data_path: &std::path::PathBuf) -> std::path::PathBuf {
        published_data_path
            .join(&self.package_name)
            .join(&self.package_uuid)
            .join(&self.package_timestamp)
    }

    pub fn get_package_name(&self) -> &str {
        &self.package_name
    }

    pub fn get_package_uuid(&self) -> &str {
        &self.package_uuid
    }

    pub fn get_package_timestamp(&self) -> &str {
        &self.package_timestamp
    }

    pub fn is_valid_package_timestamp(package_timestamp: &str) -> bool {
        package_timestamp.parse::<u64>().is_ok()
    }

    pub fn is_valid_package_uuid(package_uuid: &str) -> bool {
        // Valid if all uppercase alphanumeric
        package_uuid
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_digit(10))
    }
}

impl Eq for PackagePath {}

impl Hash for PackagePath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package_name.hash(state);
        self.package_uuid.hash(state);
        self.package_timestamp.hash(state);
    }
}

impl PartialEq for PackagePath {
    fn eq(&self, other: &Self) -> bool {
        self.package_name == other.package_name
            && self.package_uuid == other.package_uuid
            && self.package_timestamp == other.package_timestamp
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
