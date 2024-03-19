// File access to the user's Suibase installation (~/suibase)
//
// In particular, converts suibase.yaml to Rust structs.
//
use home::home_dir;
use log::info;
use std::collections::{HashMap, LinkedList};

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::basic_types::{GenericTx, ManagedElement, ManagedVec, ManagedVecU8, WorkdirIdx};

// workdir_idx are hard coded for performance.
pub const WORKDIR_IDX_MAINNET: WorkdirIdx = 0;
pub const WORKDIR_IDX_TESTNET: WorkdirIdx = 1;
pub const WORKDIR_IDX_DEVNET: WorkdirIdx = 2;
pub const WORKDIR_IDX_LOCALNET: WorkdirIdx = 3;

// List of all possible workdirs planned to be supported.
// The order is important since the position match the WORKDIR_IDX_* constants.
pub const WORKDIRS_KEYS: [&str; 4] = ["mainnet", "testnet", "devnet", "localnet"];

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Link {
    // A link in a suibase.yaml file.
    pub alias: String,
    pub selectable: bool,
    pub monitored: bool,
    pub rpc: Option<String>,
    pub metrics: Option<String>,
    pub ws: Option<String>,
    pub priority: u8,
}

impl Link {
    pub fn new(alias: String, rpc: String) -> Self {
        Self {
            alias,
            selectable: true,
            monitored: true,
            rpc: Some(rpc),
            metrics: None,
            ws: None,
            priority: u8::MAX,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct DTPService {
    // A service in a suibase.yaml file
    service_type: String,
    enabled: bool,
    client_enabled: bool,
    server_enabled: bool,
    alias: Option<String>,
    gas_address: Option<String>,
    remote_host: Option<String>,
    client_auth: Option<String>,
    server_auth: Option<String>,
    local_port: Option<u16>,

    // "Calculated" fields not in the config.
    // 0 means 'wildcard' service_type and is used to configure
    // defaults gas_address, remote_host etc...
    service_idx: u8,
}

impl DTPService {
    pub fn new(service_type: String) -> Self {
        Self {
            service_type,
            enabled: false,
            client_enabled: false,
            server_enabled: false,
            alias: None,
            gas_address: None,
            remote_host: None,
            client_auth: None,
            server_auth: None,
            local_port: None,
            service_idx: 0,
        }
    }

    pub fn service_type(&self) -> &str {
        &self.service_type
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_client_enabled(&self) -> bool {
        self.client_enabled
    }

    pub fn is_server_enabled(&self) -> bool {
        self.server_enabled
    }

    pub fn alias(&self) -> Option<&String> {
        self.alias.as_ref()
    }

    pub fn gas_address(&self) -> Option<&String> {
        self.gas_address.as_ref()
    }

    pub fn client_auth(&self) -> Option<&String> {
        self.client_auth.as_ref()
    }

    pub fn server_auth(&self) -> Option<&String> {
        self.server_auth.as_ref()
    }

    pub fn remote_host(&self) -> Option<&String> {
        self.remote_host.as_ref()
    }

    pub fn local_port(&self) -> Option<u16> {
        self.local_port
    }

    pub fn service_idx(&self) -> u8 {
        self.service_idx
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkdirUserConfig {
    // Created from parsing/merging suibase.yaml file(s) for a single workdir,
    // except for 'user_request' which is loaded from '.state/user_request'.
    user_request: Option<String>,
    user_request_start: bool, // true when user_request == "start"
    proxy_enabled: bool,
    proxy_port_number: u16,
    links_overrides: bool,
    links: HashMap<String, Link>,
    dtp_package_id: Option<String>, // Package ID of the DTP package for this workdir.
    dtp_services: LinkedList<DTPService>, // Each configured service.
    dtp_default_gas_address: Option<String>, // Pays gas when txn not related to a service.
}

impl WorkdirUserConfig {
    pub fn new() -> Self {
        Self {
            user_request: None,
            user_request_start: false,
            proxy_enabled: false,
            proxy_port_number: 0,
            links_overrides: false,
            links: HashMap::new(),
            dtp_package_id: None,
            dtp_services: LinkedList::new(),
            dtp_default_gas_address: None,
        }
    }

    pub fn user_request(&self) -> Option<&String> {
        self.user_request.as_ref()
    }

    pub fn is_user_request_start(&self) -> bool {
        self.user_request_start
    }

    pub fn is_proxy_enabled(&self) -> bool {
        self.proxy_enabled
    }

    pub fn proxy_port_number(&self) -> u16 {
        self.proxy_port_number
    }

    pub fn links_overrides(&self) -> bool {
        self.links_overrides
    }

    pub fn links(&self) -> &HashMap<String, Link> {
        &self.links
    }

    pub fn dtp_services(&self) -> &LinkedList<DTPService> {
        &self.dtp_services
    }

    pub fn dtp_default_gas_address(&self) -> Option<String> {
        self.dtp_default_gas_address.clone()
    }

    pub fn dtp_package_id(&self) -> Option<String> {
        self.dtp_package_id.clone()
    }

    pub fn dtp_service_config(
        &self,
        service_idx: u8,
        remote_host: Option<String>,
    ) -> Option<DTPService> {
        // Search in the linked list for a matching service_idx and remote_host.
        // Log the size of the dtp_services
        info!(
            "on dtp_service_config() dtp_services size: {}",
            self.dtp_services.len()
        );
        for dtp_service in &self.dtp_services {
            if dtp_service.service_idx == service_idx
                && (remote_host.is_none() || dtp_service.remote_host == remote_host)
            {
                return Some(dtp_service.clone());
            }
        }
        None
    }

    pub fn load_state_file(&mut self, path: &str) -> Result<()> {
        if let Ok(contents) = std::fs::read_to_string(path) {
            // Trim trailing newline.
            let contents = contents.trim_end().to_string();
            self.user_request_start = contents == "start";
            self.user_request = Some(contents);
        } else {
            // This means the workdir directory is not fully initialized.
            self.user_request = None;
            self.user_request_start = false;
        }
        Ok(())
    }

    pub fn load_and_merge_from_file(&mut self, path: &str) -> Result<()> {
        self.load_and_merge_from_file_internal(path, false)
    }

    pub fn load_and_merge_from_common_file(&mut self, path: &str) -> Result<()> {
        self.load_and_merge_from_file_internal(path, true)
    }

    fn load_and_merge_from_file_internal(&mut self, path: &str, common: bool) -> Result<()> {
        // This merge the config of the file with the current
        // configuration.
        //
        // For most variables, if a new value is defined in the
        // file it will overwrite.
        //
        // For most lists (e.g. links), they are merged.
        //
        // =====
        //
        // Example of suibase.yaml:
        //
        // proxy_enabled: false
        //
        // links:
        //   - alias: "localnet"
        //     rpc: "http://0.0.0.0:9000"
        //     ws: "ws://0.0.0.0:9000"
        //     priority: 10
        //   - alias: "localnet"
        //     enabled: false
        //     rpc: "http://0.0.0.0:9000"
        //
        // dtp_package_id: "0x9c0c8b2b487fd0dcc00cb070df45a82b302ba6bc8244edd85c82e1409ad430ca"
        //
        // dtp_services:
        //   - service_type: "ping"
        //     client_address: 0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462
        //
        //   - service_type: "json-rpc"
        //     client_address: 0xef6e9dd8f30dea802e0474a7996e5c772c581cc1adee45afb660f15a081d1c49
        //     remote_host: 0x6fff280505c35ab84d067f2c6a34a6182a1c4607cffea7302bcbfb7f735007ad
        //     local_port: 45000
        //
        //   - service_type: "default"
        //     client_address: 0xc7294a5cc946db818c4058c83c933ad6c28e73711bee21c7fa85553c90cb7244
        //
        let contents = std::fs::read_to_string(path)?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)?;

        // TODO: Lots of robustness could be added here...

        // proxy_enabled can be "true", "false" or "dev" for testing.
        //
        // "dev" is similar to "true" but allows for foreground execution
        // of the suibase-daemon... only the bash scripts care for this.

        // TODO Implement dtp_enabled and remove this force to false.
        self.proxy_enabled = false;

        /* For dtp-daemon... proxy is always disabled for now
        if let Some(proxy_enabled) = yaml["proxy_enabled"].as_bool() {
            self.proxy_enabled = proxy_enabled;
        }
        if let Some(proxy_enabled) = yaml["proxy_enabled"].as_str() {
            self.proxy_enabled = proxy_enabled != "false";
        }*/

        if let Some(links_overrides) = yaml["links_overrides"].as_bool() {
            // Clear all the previous links!
            self.links.clear();
            self.links_overrides = links_overrides;
        }

        // Remaining variables do not make sense in common files, so ignore them.
        // TODO: Implement warning user for bad usage...
        if common {
            return Ok(());
        }

        if let Some(proxy_port_number) = yaml["proxy_port_number"].as_u64() {
            self.proxy_port_number = proxy_port_number as u16;
        }

        if let Some(dtp_package_id) = yaml["dtp_package_id"].as_str() {
            self.dtp_package_id = Some(dtp_package_id.to_string());
        }

        if let Some(links) = yaml["links"].as_sequence() {
            for link in links {
                if let Some(alias) = link["alias"].as_str() {
                    // TODO: Consider implementing link level member merging.

                    // Default of "enabled" is true. Allow the user to disable a single link.
                    //
                    // May allow later user finer control with "selectable" and "monitored".
                    let enabled = link["enabled"].as_bool().unwrap_or(true);
                    let selectable = enabled;
                    let monitored = enabled;

                    let rpc = link["rpc"].as_str().map(|s| s.to_string()); // Optional
                    let metrics = link["metrics"].as_str().map(|s| s.to_string()); // Optional
                    let ws = link["ws"].as_str().map(|s| s.to_string()); // Optional
                    let priority = link["priority"].as_u64().unwrap_or(u64::MAX) as u8;
                    let link = Link {
                        alias: alias.to_string(),
                        selectable,
                        monitored,
                        rpc,
                        metrics,
                        ws,
                        priority,
                    };
                    // Replace if already present.
                    self.links.insert(alias.to_string(), link);
                }
            }
        }

        if let Some(services) = yaml["dtp_services"].as_sequence() {
            for service in services {
                if let Some(service_type) = service["service_type"].as_str() {
                    // Default of "enabled" is true. Allow the user to disable a single service.
                    let enabled = service["enabled"].as_bool().unwrap_or(true);

                    // TODO A ServiceType to match the Move definition.
                    // Validate that service_type is one of the following string,
                    // and map it to a service_idx.
                    //
                    //   "json-rpc" is service_idx 2
                    //   "ping" is service_idx 7
                    //   "default" is valid but does not have a service_idx.
                    let service_idx = match service_type {
                        "json-rpc" => Some(2u8),
                        "ping" => Some(7u8),
                        "default" => Some(0u8),
                        _ => None,
                    };
                    if service_idx.is_none() || !enabled {
                        continue; // Skip it.
                    }
                    let service_idx = service_idx.unwrap();

                    let gas_address = service["gas_address"].as_str().map(|s| s.to_string()); // Optional

                    if service_idx == 0 {
                        if let Some(gas_address) = gas_address {
                            self.dtp_default_gas_address = Some(gas_address);
                        }
                        continue;
                    }

                    let client_auth = service["client_auth"].as_str().map(|s| s.to_string()); // Optional
                    let server_auth = service["server_auth"].as_str().map(|s| s.to_string()); // Optional

                    let alias = service["alias"].as_str().map(|s| s.to_string()); // Optional
                    let remote_host = service["remote_host"].as_str().map(|s| s.to_string()); // Optional
                    let local_port = service["local_port"].as_u64().map(|v| v as u16); // Optional

                    let client_enabled = client_auth.is_some();
                    let server_enabled = server_auth.is_some();

                    let dtp_service = DTPService {
                        service_type: service_type.to_string(),
                        enabled,
                        client_enabled,
                        server_enabled,
                        alias,
                        gas_address,
                        remote_host,
                        client_auth,
                        server_auth,
                        local_port,
                        service_idx,
                    };

                    // Insert, and ignore duplicates.
                    if !self.dtp_services.contains(&dtp_service) {
                        info!("Added DTP service for idx: {:?}", dtp_service);
                        self.dtp_services.push_back(dtp_service);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for WorkdirUserConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Workdir {
    idx: Option<ManagedVecU8>,
    name: String,
    path: PathBuf,
    state_path: PathBuf,
    suibase_state_file: PathBuf,
    suibase_yaml_user: PathBuf,
    suibase_yaml_default: PathBuf,
}

impl Workdir {
    pub fn idx(&self) -> Option<ManagedVecU8> {
        self.idx
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    pub fn is_user_request_start(&self) -> bool {
        // Return true if suibase_state_file() exists, and if so, check if its text
        // content is "start".
        //
        // If there is any error, return false.
        //
        if let Ok(contents) = std::fs::read_to_string(self.suibase_state_file()) {
            if contents.starts_with("start") {
                return true;
            }
        }
        false
    }

    pub fn suibase_state_file(&self) -> &Path {
        &self.suibase_state_file
    }

    pub fn suibase_yaml_user(&self) -> &Path {
        &self.suibase_yaml_user
    }

    pub fn suibase_yaml_default(&self) -> &Path {
        &self.suibase_yaml_default
    }
}

#[derive(Debug)]
pub struct GlobalsWorkdirsST {
    pub workdirs: ManagedVec<Workdir>,
    suibase_home: String,
    path: PathBuf,
    suibase_yaml_common: PathBuf,

    // Variables that rarely changes and can be controlled only by
    // the common suibase.yaml (not the workdir specific suibase.yaml)
    pub suibase_web_ip: String,
    pub suibase_web_port: u16,

    pub suibase_api_ip: String,
    pub suibase_api_port: u16,

    pub dtp_api_ip: String,
    pub dtp_api_port: u16,
}

impl GlobalsWorkdirsST {
    pub fn new() -> Self {
        let home_dir = if let Some(home_dir) = home_dir() {
            home_dir
        } else {
            // The program will likely fail to further initialize, so pointing to /tmp
            // in meantime is a reasonable default/fallback safe thing to do...
            PathBuf::from("/tmp")
        };

        let suibase_home = home_dir.join("suibase");

        // Generate all the suibase paths for state and config files of each WORKDIRS_KEYS.
        let mut workdirs = ManagedVec::new();

        let workdirs_path = suibase_home.join("workdirs");

        for workdir in WORKDIRS_KEYS.iter() {
            // Paths
            let path = workdirs_path.join(workdir);

            let state_path = path.join(".state");

            // Files
            let state = state_path.join("user_request");

            let user_yaml = path.join("suibase.yaml");

            let mut default_yaml = suibase_home.join("scripts");
            default_yaml.push("defaults");
            default_yaml.push(workdir);
            default_yaml.push("suibase.yaml");

            workdirs.push(Workdir {
                idx: None,
                name: workdir.to_string(),
                path,
                state_path,
                suibase_state_file: state,
                suibase_yaml_user: user_yaml,
                suibase_yaml_default: default_yaml,
            });
        }

        let suibase_yaml_common = workdirs_path.join("common").join("suibase.yaml");

        Self {
            suibase_home: suibase_home.to_string_lossy().to_string(),
            workdirs,
            path: workdirs_path,
            suibase_yaml_common,

            // TODO Get these really from the common suibase.yaml. Hard coded for now.
            suibase_web_ip: "0.0.0.0".to_string(),
            suibase_web_port: 44380,

            suibase_api_ip: "0.0.0.0".to_string(),
            suibase_api_port: 44399,

            dtp_api_ip: "0.0.0.0".to_string(),
            dtp_api_port: 44398,
        }
    }

    pub fn suibase_home(&self) -> &str {
        &self.suibase_home
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn suibase_yaml_common(&self) -> &Path {
        &self.suibase_yaml_common
    }

    pub fn suibase_api_ip(&self) -> &str {
        &self.suibase_api_ip
    }

    pub fn suibase_api_port(&self) -> u16 {
        self.suibase_api_port
    }

    // Given a path string, find the corresponding workdir object.
    // This also works if the string is simply the workdir name (e.g. "localnet").
    pub fn find_workdir(&self, path: &str) -> Option<(WorkdirIdx, &Workdir)> {
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

    // Write access to a Workdir stored in globals.
    pub fn get_workdir_mut(&mut self, workdir_idx: WorkdirIdx) -> Option<&mut Workdir> {
        self.workdirs.get_mut(workdir_idx)
    }
}

impl Default for GlobalsWorkdirsST {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedElement for Workdir {
    fn idx(&self) -> Option<ManagedVecU8> {
        self.idx
    }

    fn set_idx(&mut self, index: Option<ManagedVecU8>) {
        self.idx = index;
    }
}

// TODO Merge Workdir into GlobalsWorkdirConfigST

// User configuration for every workdir (mostly from suibase.yaml).
#[derive(Debug)]
pub struct GlobalsWorkdirConfigST {
    pub user_config: WorkdirUserConfig,
}

impl GlobalsWorkdirConfigST {
    pub fn new() -> Self {
        Self {
            user_config: WorkdirUserConfig::new(),
        }
    }

    pub fn from(user_config: WorkdirUserConfig) -> Self {
        Self { user_config }
    }
}

impl Default for GlobalsWorkdirConfigST {
    fn default() -> Self {
        Self::new()
    }
}
