// File access to the user's Suibase installation (~/suibase)
//
// In particular, converts suibase.yaml to Rust structs.
//
use home::home_dir;
use log::info;
use std::collections::{HashMap, LinkedList};

use std::path::{Path, PathBuf};

use anyhow::Result;
use std::sync::LazyLock;

use crate::basic_types::{ClientMode, WorkdirIdx};

// workdir_idx are hard coded for performance.
pub const WORKDIR_IDX_MAINNET: WorkdirIdx = 0;
pub const WORKDIR_IDX_TESTNET: WorkdirIdx = 1;
pub const WORKDIR_IDX_DEVNET: WorkdirIdx = 2;
pub const WORKDIR_IDX_LOCALNET: WorkdirIdx = 3;

// All .state files names are hard coded for consistency.
// (must remain backward compatible).
pub const STATE_USER_REQUEST: &str = "user_request";

// List of all possible workdirs planned to be supported.
// The order is important since the position match the WORKDIR_IDX_* constants.
pub const WORKDIRS_KEYS: [&str; 4] = ["mainnet", "testnet", "devnet", "localnet"];
pub const WORKDIRS_COUNT: usize = WORKDIRS_KEYS.len();

// Utility that returns the workdir_idx for one of the WORKDIRS_KEYS
//
// This call is relatively costly, use wisely.
pub fn get_workdir_idx_by_name(workdir_name: &String) -> Option<WorkdirIdx> {
    for (idx, workdir) in WORKDIRS_KEYS.iter().enumerate() {
        if workdir_name == workdir {
            return Some(idx as WorkdirIdx);
        }
    }
    None
}

pub fn get_workdir_idx_by_path(path: &str) -> Option<WorkdirIdx> {
    for (idx, workdir) in WORKDIRS_KEYS.iter().enumerate() {
        if path.contains(workdir) {
            return Some(idx as WorkdirIdx);
        }
    }
    None
}

// Utility to get path information.
pub struct WorkdirPaths {
    // A workdir path itself (e.g. ~/suibase/workdirs/testnet )
    workdir_root_path: PathBuf,

    // Subdirectories of workdir_path
    state_path: PathBuf,
    suibase_yaml_user: PathBuf,
    suibase_yaml_default: PathBuf,

    // Path to ~/suibase/workdirs/common/suibase.yaml
    // Here for convenience even if not workdir_path specific.
    suibase_yaml_common: PathBuf,
}

struct SuibasePaths {
    home_path: PathBuf,
    home_suibase_path: PathBuf,
    workdirs_path: PathBuf,
    workdir_common_path: PathBuf,
    workdir_paths: [WorkdirPaths; WORKDIRS_COUNT],
}

// Cache the workdir paths using LazyLock
static WORKDIR_PATHS: LazyLock<SuibasePaths> = LazyLock::new(|| {
    let home_path = home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    let home_suibase_path = home_path.join("suibase");
    let workdirs_path = home_suibase_path.join("workdirs");
    let workdirs_common_path = workdirs_path.join("common");

    let workdir_paths = core::array::from_fn(|idx| {
        let workdir_name = WORKDIRS_KEYS[idx];
        let workdir_root_path = workdirs_path.join(workdir_name);
        let state_path = workdir_root_path.join(".state");
        let suibase_yaml_user = workdir_root_path.join("suibase.yaml");
        let suibase_yaml_default = home_suibase_path
            .join("scripts")
            .join("defaults")
            .join(workdir_name)
            .join("suibase.yaml");
        let suibase_yaml_common = workdirs_path.join("common").join("suibase.yaml");

        WorkdirPaths {
            workdir_root_path,
            state_path,
            suibase_yaml_user,
            suibase_yaml_default,
            suibase_yaml_common,
        }
    });

    SuibasePaths {
        home_path,
        home_suibase_path,
        workdirs_path,
        workdir_common_path: workdirs_common_path,
        workdir_paths,
    }
});

pub fn get_home_path() -> &'static Path {
    &WORKDIR_PATHS.home_path
}

pub fn get_home_suibase_path() -> &'static Path {
    &WORKDIR_PATHS.home_suibase_path
}

pub fn get_workdirs_path() -> &'static Path {
    // e.g. /home/user/suibase/workdirs
    &WORKDIR_PATHS.workdirs_path
}

pub fn get_workdir_paths(workdir_idx: WorkdirIdx) -> &'static WorkdirPaths {
    // Struct conveniently providing all subdir paths for a workdir.
    // Careful. Will rightfully panic if passing invalid workdir_idx.
    &WORKDIR_PATHS.workdir_paths[workdir_idx as usize]
}

pub fn get_workdir_common_path() -> &'static Path {
    // e.g. /home/user/suibase/workdirs/common
    &WORKDIR_PATHS.workdir_common_path
}

impl WorkdirPaths {
    pub fn workdir_root_path(&self) -> &Path {
        &self.workdir_root_path
    }

    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    pub fn suibase_yaml_user(&self) -> &Path {
        &self.suibase_yaml_user
    }

    pub fn suibase_yaml_default(&self) -> &Path {
        &self.suibase_yaml_default
    }

    pub fn suibase_yaml_common(&self) -> &Path {
        &self.suibase_yaml_common
    }

    pub fn state_file_path(&self, state_name: &str) -> PathBuf {
        self.state_path.join(state_name)
    }
}

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
    pub max_per_secs: Option<u32>, // Rate limit: maximum requests per second (None = unlimited)
    pub max_per_min: Option<u32>,  // Rate limit: maximum requests per minute (None = unlimited)
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
            max_per_secs: None, // Default: no rate limit
            max_per_min: None,  // Default: no rate limit
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
    // Created from parsing/merging suibase.yaml file(s) for a single workdir.
    // Also includes .state files variables.
    user_request: Option<String>,
    user_request_start: bool, // true when user_request == "start"
    proxy_enabled: bool,
    proxy_port_number: u16,
    links_overrides: bool,
    links: HashMap<String, Link>,
    dtp_package_id: Option<String>, // Package ID of the DTP package for this workdir.
    dtp_services: LinkedList<DTPService>, // Each configured service.
    dtp_default_gas_address: Option<String>, // Pays gas when txn not related to a service.
    autocoins_enabled: bool,
    autocoins_address: Option<String>,
    autocoins_mode: ClientMode,
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
            autocoins_enabled: false,
            autocoins_address: None,
            autocoins_mode: ClientMode::Unknown,
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

    pub fn is_autocoins_enabled(&self) -> bool {
        self.autocoins_enabled
    }

    pub fn autocoins_address(&self) -> Option<String> {
        self.autocoins_address.clone()
    }

    pub fn autocoins_mode(&self) -> ClientMode {
        self.autocoins_mode
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

    fn load_state_string(workdir_paths: &WorkdirPaths, state_name: &str) -> Option<String> {
        let state_file_path = &workdir_paths.state_file_path(state_name);
        let contents = std::fs::read_to_string(state_file_path).ok()?;
        Some(contents.trim_end().to_string())
    }

    pub fn load_state_files(&mut self, workdir_paths: &WorkdirPaths) -> Result<()> {
        if let Some(contents) = Self::load_state_string(workdir_paths, STATE_USER_REQUEST) {
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
        //     rpc: "http://localhost:9000"
        //     ws: "ws://localhost:9000"
        //     priority: 10
        //   - alias: "localnet"
        //     enabled: false
        //     rpc: "http://localhost:9000"
        //
        // dtp_package_id: "0x9c0c8..."
        //
        // dtp_services:
        //   - service_type: "ping"
        //     client_address: 0xf7ae...
        //
        //   - service_type: "json-rpc"
        //     client_address: 0xef6e...
        //     remote_host: 0x6fff2...
        //     local_port: 45000
        //
        //   - service_type: "default"
        //     client_address: 0xc729...
        //
        let contents = std::fs::read_to_string(path)?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)?;

        // TODO: Lots of robustness could be added here...

        // proxy_enabled can be "true", "false" or "dev" for testing.
        //
        // "dev" is similar to "true" but allows for foreground execution
        // of the suibase-daemon... only the bash scripts care for this.
        if let Some(proxy_enabled) = yaml["proxy_enabled"].as_bool() {
            self.proxy_enabled = proxy_enabled;
        } else if let Some(proxy_enabled) = yaml["proxy_enabled"].as_str() {
            self.proxy_enabled = proxy_enabled != "false";
        }

        // autocoins_enabled can be "true" or "false".
        if let Some(autocoins_enabled) = yaml["autocoins_enabled"].as_bool() {
            self.autocoins_enabled = autocoins_enabled;
        } else if let Some(autocoins_enabled) = yaml["autocoins_enabled"].as_str() {
            self.autocoins_enabled = autocoins_enabled != "false";
        }

        if let Some(mode) = yaml["autocoins_mode"].as_str() {
            match mode.to_lowercase().as_str() {
                "stage" => self.autocoins_mode = ClientMode::Stage,
                "test" => self.autocoins_mode = ClientMode::Test,
                "public" => self.autocoins_mode = ClientMode::Public,
                "unknown" => self.autocoins_mode = ClientMode::Unknown,
                _ => {
                    // Invalid mode, default to Unknown
                    self.autocoins_mode = ClientMode::Unknown;
                }
            }
        }

        // autocoins_address is a hex string, e.g. "0x1234..." with 64 hex digits. The
        // 0x is optional. autocoins_address remains None if not present in the file
        // or can't be parsed.
        let mut address_is_valid = false;
        if let Some(autocoins_address) = yaml["autocoins_address"].as_str() {
            let normalized_address = if autocoins_address.starts_with("0x") {
                &autocoins_address[2..]
            } else {
                autocoins_address
            };
            if normalized_address.len() == 64 {
                // Verify it's valid hex
                let is_valid_hex = normalized_address.chars().all(|c| {
                    c.is_ascii_digit() || ('a'..='f').contains(&c) || ('A'..='F').contains(&c)
                });

                if is_valid_hex {
                    // Ensure consistent format with 0x prefix
                    let formatted_address = format!("0x{}", normalized_address.to_lowercase());
                    self.autocoins_address = Some(formatted_address);
                    address_is_valid = true;
                }
            }
        }
        if !address_is_valid {
            self.autocoins_address = None;
        }

        let mut clear_links = false;
        if let Some(links_overrides) = yaml["links_overrides"].as_bool() {
            clear_links = true;
            self.links_overrides = links_overrides;
        } else if let Some(links_overrides) = yaml["links_overrides"].as_str() {
            clear_links = true;
            self.links_overrides = links_overrides != "false";
        }

        if clear_links {
            // Clear all the previous links.
            self.links.clear();
        }

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
                    // Also support separate "selectable" and "monitored" flags for finer control.
                    let enabled = link["enabled"].as_bool().unwrap_or(true);
                    let selectable = link["selectable"].as_bool().unwrap_or(enabled);
                    let monitored = link["monitored"].as_bool().unwrap_or(enabled);

                    let rpc = link["rpc"].as_str().map(|s| s.to_string()); // Optional
                    let metrics = link["metrics"].as_str().map(|s| s.to_string()); // Optional
                    let ws = link["ws"].as_str().map(|s| s.to_string()); // Optional
                    let priority = link["priority"].as_u64().unwrap_or(u64::MAX) as u8;
                    let max_per_secs = link["max_per_secs"].as_u64().map(|v| v as u32); // Optional rate limit
                    let max_per_min = link["max_per_min"].as_u64().map(|v| v as u32); // Optional rate limit
                    let link = Link {
                        alias: alias.to_string(),
                        selectable,
                        monitored,
                        rpc,
                        metrics,
                        ws,
                        priority,
                        max_per_secs,
                        max_per_min,
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

// Include unit tests for configuration parsing
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_link_struct_defaults() {
        let link = Link::new("test".to_string(), "http://localhost:9000".to_string());

        assert_eq!(link.alias, "test");
        assert_eq!(link.selectable, true);
        assert_eq!(link.monitored, true);
        assert_eq!(link.rpc, Some("http://localhost:9000".to_string()));
        assert_eq!(link.metrics, None);
        assert_eq!(link.ws, None);
        assert_eq!(link.priority, u8::MAX);
        assert_eq!(link.max_per_secs, None); // Default: no rate limit
        assert_eq!(link.max_per_min, None); // Default: no rate limit
    }

    #[test]
    fn test_yaml_parsing_with_dual_rate_limits() {
        let yaml_content = r#"
proxy_enabled: true
links:
  - alias: "testnet_rpc"
    rpc: "https://fullnode.testnet.sui.io:443"
    max_per_secs: 100
    max_per_min: 5000
    priority: 10
  - alias: "localnet_rpc"  
    rpc: "http://localhost:9000"
    max_per_secs: 50
    priority: 20
  - alias: "unlimited_rpc"
    rpc: "http://localhost:8000"
    priority: 30
  - alias: "qpm_only_rpc"
    rpc: "http://localhost:7000"
    max_per_min: 3000
    priority: 40
"#;

        // Create a temporary file with the YAML content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();
        let result = config.load_and_merge_from_file(temp_path);

        assert!(result.is_ok(), "Failed to parse YAML: {:?}", result.err());
        assert_eq!(config.is_proxy_enabled(), true);

        let links = config.links();
        assert_eq!(links.len(), 4);

        // Test testnet_rpc link with both rate limits
        let testnet_link = links.get("testnet_rpc").unwrap();
        assert_eq!(testnet_link.alias, "testnet_rpc");
        assert_eq!(
            testnet_link.rpc,
            Some("https://fullnode.testnet.sui.io:443".to_string())
        );
        assert_eq!(testnet_link.max_per_secs, Some(100));
        assert_eq!(testnet_link.max_per_min, Some(5000));
        assert_eq!(testnet_link.priority, 10);

        // Test localnet_rpc link with QPS only
        let localnet_link = links.get("localnet_rpc").unwrap();
        assert_eq!(localnet_link.alias, "localnet_rpc");
        assert_eq!(localnet_link.rpc, Some("http://localhost:9000".to_string()));
        assert_eq!(localnet_link.max_per_secs, Some(50));
        assert_eq!(localnet_link.max_per_min, None);
        assert_eq!(localnet_link.priority, 20);

        // Test unlimited_rpc link without any rate limits
        let unlimited_link = links.get("unlimited_rpc").unwrap();
        assert_eq!(unlimited_link.alias, "unlimited_rpc");
        assert_eq!(
            unlimited_link.rpc,
            Some("http://localhost:8000".to_string())
        );
        assert_eq!(unlimited_link.max_per_secs, None);
        assert_eq!(unlimited_link.max_per_min, None);
        assert_eq!(unlimited_link.priority, 30);

        // Test qpm_only_rpc link with QPM only
        let qpm_link = links.get("qpm_only_rpc").unwrap();
        assert_eq!(qpm_link.alias, "qpm_only_rpc");
        assert_eq!(qpm_link.rpc, Some("http://localhost:7000".to_string()));
        assert_eq!(qpm_link.max_per_secs, None);
        assert_eq!(qpm_link.max_per_min, Some(3000));
        assert_eq!(qpm_link.priority, 40);
    }

    #[test]
    fn test_yaml_parsing_max_per_min_only() {
        let yaml_content = r#"
links:
  - alias: "qpm_rpc"
    rpc: "http://localhost:9000"
    max_per_min: 2400
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();
        let result = config.load_and_merge_from_file(temp_path);

        assert!(result.is_ok());

        let links = config.links();
        let qpm_link = links.get("qpm_rpc").unwrap();
        assert_eq!(qpm_link.max_per_secs, None);
        assert_eq!(qpm_link.max_per_min, Some(2400));
    }

    #[test]
    fn test_yaml_parsing_zero_rate_limit() {
        let yaml_content = r#"
links:
  - alias: "zero_rate_rpc"
    rpc: "http://localhost:9000"
    max_per_secs: 0
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();
        let result = config.load_and_merge_from_file(temp_path);

        assert!(result.is_ok());

        let links = config.links();
        let zero_rate_link = links.get("zero_rate_rpc").unwrap();
        assert_eq!(zero_rate_link.max_per_secs, Some(0)); // 0 should be preserved (means unlimited)
        assert_eq!(zero_rate_link.max_per_min, None);
    }

    #[test]
    fn test_yaml_parsing_large_rate_limit() {
        let yaml_content = r#"
links:
  - alias: "high_rate_rpc"
    rpc: "http://localhost:9000"
    max_per_secs: 4294967295
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();
        let result = config.load_and_merge_from_file(temp_path);

        assert!(result.is_ok());

        let links = config.links();
        let high_rate_link = links.get("high_rate_rpc").unwrap();
        assert_eq!(high_rate_link.max_per_secs, Some(u32::MAX)); // Should handle max u32 value
        assert_eq!(high_rate_link.max_per_min, None);
    }

    #[test]
    fn test_yaml_parsing_invalid_rate_limit() {
        let yaml_content = r#"
links:
  - alias: "invalid_rate_rpc"
    rpc: "http://localhost:9000"
    max_per_secs: "not_a_number"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();
        let result = config.load_and_merge_from_file(temp_path);

        assert!(result.is_ok()); // Should not fail, just ignore invalid value

        let links = config.links();
        let invalid_rate_link = links.get("invalid_rate_rpc").unwrap();
        assert_eq!(invalid_rate_link.max_per_secs, None); // Should be None for invalid values
        assert_eq!(invalid_rate_link.max_per_min, None);
    }

    #[test]
    fn test_yaml_parsing_negative_rate_limit() {
        let yaml_content = r#"
links:
  - alias: "negative_rate_rpc"
    rpc: "http://localhost:9000"
    max_per_secs: -1
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();
        let result = config.load_and_merge_from_file(temp_path);

        assert!(result.is_ok()); // Should not fail, just ignore negative value

        let links = config.links();
        let negative_rate_link = links.get("negative_rate_rpc").unwrap();
        assert_eq!(negative_rate_link.max_per_secs, None); // Should be None for negative values
        assert_eq!(negative_rate_link.max_per_min, None);
    }

    #[test]
    fn test_configuration_merging_preserves_rate_limits() {
        // Create first config file
        let yaml_content1 = r#"
links:
  - alias: "server1"
    rpc: "http://server1:9000"
    max_per_secs: 100
  - alias: "server2"
    rpc: "http://server2:9000"
    max_per_secs: 200
"#;

        // Create second config file that overrides server1 but adds server3
        let yaml_content2 = r#"
links:
  - alias: "server1"
    rpc: "http://server1-updated:9000"
    max_per_secs: 150
  - alias: "server3"
    rpc: "http://server3:9000"
    max_per_secs: 300
"#;

        let mut temp_file1 = NamedTempFile::new().unwrap();
        temp_file1.write_all(yaml_content1.as_bytes()).unwrap();
        let temp_path1 = temp_file1.path().to_str().unwrap();

        let mut temp_file2 = NamedTempFile::new().unwrap();
        temp_file2.write_all(yaml_content2.as_bytes()).unwrap();
        let temp_path2 = temp_file2.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();

        // Load first file
        let result1 = config.load_and_merge_from_file(temp_path1);
        assert!(result1.is_ok());

        // Load second file (should merge)
        let result2 = config.load_and_merge_from_file(temp_path2);
        assert!(result2.is_ok());

        let links = config.links();
        assert_eq!(links.len(), 3);

        // server1 should be updated
        let server1 = links.get("server1").unwrap();
        assert_eq!(server1.rpc, Some("http://server1-updated:9000".to_string()));
        assert_eq!(server1.max_per_secs, Some(150));
        assert_eq!(server1.max_per_min, None);

        // server2 should remain from first config
        let server2 = links.get("server2").unwrap();
        assert_eq!(server2.rpc, Some("http://server2:9000".to_string()));
        assert_eq!(server2.max_per_secs, Some(200));
        assert_eq!(server2.max_per_min, None);

        // server3 should be added from second config
        let server3 = links.get("server3").unwrap();
        assert_eq!(server3.rpc, Some("http://server3:9000".to_string()));
        assert_eq!(server3.max_per_secs, Some(300));
        assert_eq!(server3.max_per_min, None);
    }

    #[test]
    fn test_links_overrides_clears_rate_limits() {
        // Create first config file with links
        let yaml_content1 = r#"
links:
  - alias: "server1"
    rpc: "http://server1:9000"
    max_per_secs: 100
"#;

        // Create second config file with links_overrides: true
        let yaml_content2 = r#"
links_overrides: true
links:
  - alias: "server2"
    rpc: "http://server2:9000"
    max_per_secs: 200
"#;

        let mut temp_file1 = NamedTempFile::new().unwrap();
        temp_file1.write_all(yaml_content1.as_bytes()).unwrap();
        let temp_path1 = temp_file1.path().to_str().unwrap();

        let mut temp_file2 = NamedTempFile::new().unwrap();
        temp_file2.write_all(yaml_content2.as_bytes()).unwrap();
        let temp_path2 = temp_file2.path().to_str().unwrap();

        let mut config = WorkdirUserConfig::new();

        // Load first file
        let result1 = config.load_and_merge_from_file(temp_path1);
        assert!(result1.is_ok());
        assert_eq!(config.links().len(), 1);

        // Load second file with overrides (should clear previous links)
        let result2 = config.load_and_merge_from_file(temp_path2);
        assert!(result2.is_ok());

        let links = config.links();
        assert_eq!(links.len(), 1); // Should only have server2
        assert!(links.get("server1").is_none()); // server1 should be cleared

        let server2 = links.get("server2").unwrap();
        assert_eq!(server2.max_per_secs, Some(200));
        assert_eq!(server2.max_per_min, None);
        assert!(config.links_overrides());
    }
}

/*
#[derive(Debug)]
pub struct GlobalsWorkdirsST {
    //pub workdirs: ManagedVec<Workdir>,
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
            let suibase_yaml_user = path.join("suibase.yaml");
            let state_user_request = state_path.join("user_request");
            let state_autocoins_address = state_path.join("autocoins_address");
            let state_autocoins_last_deposit = state_path.join("autocoins_last_deposit");
            let state_autocoins_deposited = state_path.join("autocoins_deposited");

            let mut suibase_yaml_default = suibase_home.join("scripts");
            suibase_yaml_default.push("defaults");
            suibase_yaml_default.push(workdir);
            suibase_yaml_default.push("suibase.yaml");

            workdirs.push(Workdir {
                idx: None,
                name: workdir.to_string(),
                path,
                state_path,
                state_user_request,
                state_autocoins_address,
                state_autocoins_last_deposit,
                state_autocoins_deposited,
                suibase_yaml_user,
                suibase_yaml_default,
            });
        }

        let suibase_yaml_common = workdirs_path.join("common").join("suibase.yaml");

        Self {
            suibase_home: suibase_home.to_string_lossy().to_string(),
            path: workdirs_path,
            suibase_yaml_common,

            // TODO Get these really from the common suibase.yaml. Hard coded for now.
            suibase_web_ip: "localhost".to_string(),
            suibase_web_port: 44380,

            suibase_api_ip: "localhost".to_string(),
            suibase_api_port: 44399,

            dtp_api_ip: "localhost".to_string(),
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

    pub fn get_workdir(&self, workdir_idx: WorkdirIdx) -> Option<&Workdir> {
        self.workdirs.get(workdir_idx)
    }


    // Utility that returns the workdir_idx from the globals
    // using an exact workdir_name.
    //
    // This is a multi-thread safe call (will get the proper
    // lock on the globals).
    //
    // This is a relatively costly call, use wisely.
    pub async fn get_workdir_idx_by_name(
        globals: &Globals,
        workdir_name: &String,
    ) -> Option<WorkdirIdx> {
        let workdirs_guard = globals.workdirs.read().await;
        let workdirs = &*workdirs_guard;
        let workdirs_vec = &workdirs.workdirs;
        for (workdir_idx, workdir) in workdirs_vec.iter() {
            if workdir.name() == workdir_name {
                return Some(workdir_idx);
            }
        }
        None
    }

    // Utility that return a clone of the global Workdir for a given workdir_idx.
    // Multi-thread safe.
    // This is a relatively costly call, use wisely.
    pub async fn get_workdir_by_idx(globals: &Globals, workdir_idx: WorkdirIdx) -> Option<Workdir> {
        let workdirs_guard = globals.workdirs.read().await;
        let workdirs = &*workdirs_guard;
        let workdirs_vec = &workdirs.workdirs;
        if let Some(workdir) = workdirs_vec.get(workdir_idx) {
            return Some(workdir.clone());
        }
        None
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
*/

// User configuration for every workdir (mostly from suibase.yaml).
#[derive(Debug)]
pub struct GlobalsWorkdirConfigST {
    pub workdir_idx: WorkdirIdx,

    pub user_config: WorkdirUserConfig,

    // Variables that rarely changes and can be controlled only by
    // the common suibase.yaml (not the workdir specific suibase.yaml)
    pub suibase_web_ip: String,
    pub suibase_web_port: u16,

    pub suibase_api_ip: String,
    pub suibase_api_port: u16,

    pub dtp_api_ip: String,
    pub dtp_api_port: u16,
}

impl GlobalsWorkdirConfigST {
    pub fn new(workdir_idx: WorkdirIdx) -> Self {
        Self {
            workdir_idx,

            user_config: WorkdirUserConfig::new(),
            // TODO Get these really from the common suibase.yaml. Hard coded for now.
            // Consider to move these to a structure not specific to a workdir.
            suibase_web_ip: "localhost".to_string(),
            suibase_web_port: 44380,

            suibase_api_ip: "localhost".to_string(),
            suibase_api_port: 44399,

            dtp_api_ip: "localhost".to_string(),
            dtp_api_port: 44398,
        }
    }

    /*
    pub fn is_user_request_start(&self) -> bool {
        // Return true if suibase_state_file() exists, and if so, check if its text
        // content is "start".
        //
        // If there is any error, return false.
        //
        if let Ok(contents) = std::fs::read_to_string(&self.state_user_request_path) {
            if contents.starts_with("start") {
                return true;
            }
        }
        false
    }*/
}
