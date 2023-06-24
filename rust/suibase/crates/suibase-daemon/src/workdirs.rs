// Workdirs and paths related to the user's installation.
//
// Focus is on converting suibase.yaml to roughly matching Rust structs.
//
use home::home_dir;
use std::collections::HashMap;

use crate::basic_types::*;
use std::path::PathBuf;

use anyhow::Result;

// List of workdir planned to be always supported.
pub const WORKDIRS_KEYS: [&str; 4] = ["mainnet", "testnet", "devnet", "localnet"];

pub struct Link {
    // A link in a suibase.yaml file.
    pub alias: String,
    pub enabled: bool,
    pub rpc: Option<String>,
    pub ws: Option<String>,
    pub priority: u8,
}
pub struct WorkdirProxyConfig {
    // Created from parsing/merging suibase.yaml file(s) for a single workdir.
    pub links: HashMap<String, Link>, // alias is also the key. TODO Look into Hashset?
    pub links_overrides: bool,
    pub proxy_port_number: u16,
}

impl WorkdirProxyConfig {
    pub fn new() -> Self {
        Self {
            links: HashMap::new(),
            links_overrides: false,
            proxy_port_number: 0,
        }
    }

    pub fn links_overrides(&self) -> bool {
        self.links_overrides
    }

    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        // Example of suibase.yaml:
        //
        // links:
        //  - alias: "localnet"
        //    rpc: "http://0.0.0.0:9000"
        //    ws: "ws://0.0.0.0:9000"
        //    priority: 1
        //  - alias: "localnet"
        //    rpc: "http://0.0.0.0:9000"
        //    ws: "ws://0.0.0.0:9000"
        //    priority: 2
        let contents = std::fs::read_to_string(path)?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)?;

        // TODO: Lots of robustness needed to be added here...
        if let Some(proxy_port_number) = yaml["proxy_port_number"].as_u64() {
            self.proxy_port_number = proxy_port_number as u16;
        }

        if let Some(links_overrides) = yaml["links_overrides"].as_bool() {
            self.links_overrides = links_overrides;
        }

        if let Some(links) = yaml["links"].as_sequence() {
            for link in links {
                // TODO: Lots of robustness needed to be added here...
                if let Some(alias) = link["alias"].as_str() {
                    // Purpose of "enabled" is actually to disable a link... so if not present, default
                    // to enabled.
                    let enabled = link["enabled"].as_bool().unwrap_or_else(|| true);
                    let rpc = link["rpc"].as_str().map(|s| s.to_string()); // Optional
                    let ws = link["ws"].as_str().map(|s| s.to_string()); // Optional
                                                                         // Should instead remove all alpha, do absolute value, and clamp to 1-255.
                    let priority = link["priority"].as_u64().unwrap_or_else(|| u64::MAX) as u8;
                    let link = Link {
                        alias: alias.to_string(),
                        enabled,
                        rpc,
                        ws,
                        priority,
                    };

                    self.links.insert(alias.to_string(), link);
                }
            }
        }

        Ok(())
    }
}

pub struct Workdir {
    managed_idx: Option<ManagedVecUSize>,
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
    pub workdirs: ManagedVec<Workdir>,
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
        let mut workdirs = ManagedVec::new();

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

            workdirs.push(Workdir {
                managed_idx: None,
                name: workdir.to_string(),
                suibase_yaml_default: default.to_string_lossy().to_string(),
                suibase_yaml_user: user.to_string_lossy().to_string(),
            });
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
    pub fn find_workdir(&self, path: &str) -> Option<(ManagedVecUSize, &Workdir)> {
        // Remove the home_dir from the path.
        let path = path.trim_start_matches(&self.suibase_home);
        let path = path.trim_start_matches("/scripts/defaults/");
        let path = path.trim_start_matches("/workdirs/");
        for (workdir_idx, workdir) in self.workdirs.iter() {
            if path.starts_with(workdir.name()) {
                return Some((workdir_idx, workdir));
            }
        }
        None
    }
}

impl ManagedElement for Workdir {
    fn managed_idx(&self) -> Option<ManagedVecUSize> {
        self.managed_idx
    }

    fn set_managed_idx(&mut self, index: Option<ManagedVecUSize>) {
        self.managed_idx = index;
    }
}
