// Built-in workdirs and paths related to the user's installation.
//
use home::home_dir;
use std::collections::HashMap;

use std::path::PathBuf;

// List of workdir planned to be always supported.
pub const WORKDIRS_KEYS: [&str; 4] = ["localnet", "devnet", "testnet", "mainnet"];

pub struct Workdir {
    name: String,
    suibase_yaml_default: String,
    suibase_yaml_user: String,
}

impl Workdir {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn suibase_yaml_default(&self) -> &str {
        &self.suibase_yaml_default
    }
    pub fn suibase_yaml_user(&self) -> &str {
        &self.suibase_yaml_user
    }
}

pub struct Workdirs {
    pub workdirs: HashMap<String, Workdir>,
    suibase_home: String,
}

impl Workdirs {
    pub fn new() -> Self {
        let home_dir = if let Some(home_dir) = home_dir() {
            home_dir
        } else {
            // The program will likely fail to further initialize, so pointing to /tmp
            // in meantime is a reasonable default/fallback safe thing to do...
            PathBuf::from("/tmp")
        };

        // Generate all the suibase paths for loading the config files of each WORKDIRS_KEYS.
        let mut workdirs = HashMap::new();

        for workdir in WORKDIRS_KEYS.iter() {
            let mut default = home_dir.clone();
            default.push("suibase");
            default.push("scripts");
            default.push("defaults");
            default.push(workdir);
            default.push("suibase.yaml");

            let mut user = home_dir.clone();
            user.push("suibase");
            user.push("workdirs");
            user.push(workdir);
            user.push("suibase.yaml");

            workdirs.insert(
                workdir.to_string(),
                Workdir {
                    name: workdir.to_string(),
                    suibase_yaml_default: default.to_string_lossy().to_string(),
                    suibase_yaml_user: user.to_string_lossy().to_string(),
                },
            );
        }

        let mut suibase_home = home_dir.clone();
        suibase_home.push("suibase");

        Self {
            suibase_home: suibase_home.to_string_lossy().to_string(),
            workdirs,
        }
    }

    pub fn suibase_home(&self) -> &str {
        &self.suibase_home
    }

    // Given a path string, find the corresponding workdir object.
    pub fn find_workdir(&self, path: &str) -> Option<&Workdir> {
        // Remove the home_dir from the path.
        let path = path.trim_start_matches(&self.suibase_home);
        let path = path.trim_start_matches("/scripts/defaults/");
        let path = path.trim_start_matches("/workdirs/");
        for (workdir_name, workdir) in self.workdirs.iter() {
            if path.starts_with(workdir_name) {
                return Some(workdir);
            }
        }
        None
    }
}
