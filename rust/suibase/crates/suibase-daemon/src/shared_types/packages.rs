use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

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

    pub fn get_path<P: AsRef<Path>>(&self, published_data_path: P) -> PathBuf {
        published_data_path
            .as_ref()
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
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
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
