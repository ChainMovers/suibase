use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
};

use crate::shared_types::PackagePath;

// Defines the JSON-RPC API.
//
// Design:
//
// The API defined here is registered and served by jsonrpsee  (See api_server.rs).
//
// This is a thin layers and most of the heavy lifting is done in other modules.
//
// When doing a request that can "mutate" the process (other than API statistics), a message is emit
// toward the AdminController which will perform the mutation and emit a response with a tokio
// OneShot channel.
//
// This serialization of mutations helps minimizing multi-threading complexity.
//
// All *successful" JSON responses have a required "Header" field for data versioning.
//
use super::{def_header::Header, VersionedEq};
use jsonrpsee::core::RpcResult;
use jsonrpsee_proc_macros::rpc;

use schemars::JsonSchema;
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use serde_with::serde_as;

#[serde_as]
#[derive(Clone, Default, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkStats {
    // The alias of the link, as specified in the config file.
    pub alias: String,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub status: String, // Empty string, "OK" or "DOWN"

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub health_pct: String,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub load_pct: String,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub resp_time: String,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub success_pct: String,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub error_info: String, // Sometime more info when DOWN.

    // Rate limiting statistics
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub qps: Option<String>, // Current average QPS (formatted)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub qpm: Option<String>, // Current average QPM (formatted)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rate_limit_count: Option<String>, // Cumulative LIMIT count (formatted)
    
    // Raw values for data consumers
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub qps_raw: Option<u32>, // Raw QPS value
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub qpm_raw: Option<u32>, // Raw QPM value
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rate_limit_count_raw: Option<u64>, // Raw LIMIT count

    // Configuration flags for testing/debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selectable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")] 
    pub monitored: Option<bool>,
    
    // Rate limit configuration (only shown in debug mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_secs: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_min: Option<u32>,
}

impl LinkStats {
    pub fn new(alias: String) -> Self {
        LinkStats {
            alias,
            ..Default::default()
        }
    }
}

#[serde_as]
#[derive(Clone, Default, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinksSummary {
    // Each request counted only once, even when retried.
    pub success_on_first_attempt: u64,
    pub success_on_retry: u64,
    pub fail_network_down: u64,
    pub fail_bad_request: u64,
    pub fail_others: u64,
}

impl LinksSummary {
    pub fn new() -> Self {
        Self::default()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinksResponse {
    pub header: Header,

    pub status: String, // This is a single word combined "Multi-Link status". Either "OK" or "DOWN".

    pub info: String, // More details about the status (e.g. '50% degraded', 'all servers down', etc...)

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<LinksSummary>,

    // List of links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<LinkStats>>,

    // This is the output when the option 'display' is true.
    // Will also change the default to false for the summary/links output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    // This is the output when the option 'debug' is true.
    // Will also change the default to true for the summary/links/display output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,

}

impl LinksResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            status: "DISABLED".to_string(),
            info: "INITIALIZING".to_string(),
            summary: None,
            links: None,
            display: None,
            debug: None,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub header: Header,
    pub info: String, // "Success" or info on failure.
}

impl InfoResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            info: "Unknown Error".to_string(),
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusService {
    pub label: String, // "Localnet process", "Proxy server", "Multi-link RPC" etc...
    pub status: Option<String>, // OK, DOWN, DEGRADED
    pub status_info: Option<String>, // Info related to status.
    pub help_info: Option<String>, // Short help info (e.g. the faucet URL)
    pub pid: Option<u64>,
}

impl StatusService {
    pub fn new(label: String) -> Self {
        Self {
            label,
            status: None,
            status_info: None,
            help_info: None,
            pid: None,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkdirStatusResponse {
    pub header: Header,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>, // This is a single word combined "Multi-Link status". Either "OK" or "DOWN".

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_info: Option<String>, // More details about the status (e.g. '50% degraded', 'internal error', etc...)

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_version: Option<String>,

    // Finer grain status for each process/feature/service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<StatusService>>,
}

impl WorkdirStatusResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            status: None,
            status_info: None,
            client_version: None,
            network_version: None,
            services: None,
        }
    }
}

impl Default for WorkdirStatusResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionedEq for WorkdirStatusResponse {
    fn versioned_eq(&self, other: &Self) -> bool {
        // Purposely do not include header in the comparison.
        self.status == other.status
            && self.status_info == other.status_info
            && self.client_version == other.client_version
            && self.network_version == other.network_version
            && self.services == other.services
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuiEvents {
    pub message: String,
    pub timestamp: String,
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkdirSuiEventsResponse {
    pub header: Header,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<SuiEvents>>,
}

impl WorkdirSuiEventsResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            events: None,
        }
    }
}

impl Default for WorkdirSuiEventsResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub header: Header,
    pub result: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
}

impl SuccessResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            result: false,
            info: None,
        }
    }
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuiObjectType {
    // Note: Use one letter label here to keep the JSON small.
    #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
    package_id: Option<String>, //Package ID. Hexa (without 0x). Assume "self" if None.
    #[serde(rename = "m")]
    module: String,
    #[serde(rename = "n")]
    name: String,
}

impl SuiObjectType {
    pub fn new(package_id: Option<String>, module: String, name: String) -> Self {
        Self {
            package_id,
            module,
            name,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuiObjectInstance {
    // Note: Use one letter label here to keep the JSON small.
    #[serde(rename = "i")]
    object_id: String, // Object ID
    #[serde(rename = "t", skip_serializing_if = "Option::is_none")]
    object_type: Option<SuiObjectType>,
}

impl SuiObjectInstance {
    pub fn new(object_id: String, object_type: Option<SuiObjectType>) -> Self {
        Self {
            object_id,
            object_type,
        }
    }
    pub fn object_id(&self) -> &str {
        &self.object_id
    }
}

/*
mod package_path_serializer {
    use super::PackagePath;
    use serde::{self, Serializer};

    pub fn serialize<S>(package_path: &PackagePath, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!(
            "{}{}",
            package_path.get_package_name(),
            package_path.get_package_timestamp()
        );
        serializer.serialize_str(&s)
    }
}*/

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageInstance {
    pid: String, // Package ID. Hexa (no 0x)
    name: String,
    ts: String, // 64 bits Epoch Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>, // Hexa account address (no 0x)
    init: Option<Vec<SuiObjectInstance>>,

    #[serde(skip)]
    package_path: PackagePath, // Conveniently contains the UUID.
}

impl PackageInstance {
    pub fn new(package_id: String, package_path: PackagePath) -> Self {
        // Make sure does not have whitespaces, quotes or leading 0x.
        let package_id = package_id
            .trim()
            .replace("\"", "")
            .replace("'", "")
            .trim_start_matches("0x")
            .to_string();
        Self {
            pid: package_id,
            name: package_path.get_package_name().to_string(),
            ts: package_path.get_package_timestamp().to_string(),
            owner: None,
            init: None,
            package_path,
        }
    }

    pub fn set_package_owner(&mut self, package_owner: String) {
        self.owner = Some(package_owner);
    }

    pub fn set_init_objects(&mut self, init_objects: Vec<SuiObjectInstance>) {
        self.init = Some(init_objects);
    }

    pub fn get_package_name(&self) -> &str {
        &self.name
    }

    pub fn get_package_timestamp(&self) -> &str {
        &self.ts
    }

    pub fn get_package_path(&self) -> &PackagePath {
        &self.package_path
    }

    pub fn get_package_id(&self) -> &str {
        &self.pid
    }

    pub fn get_package_uuid(&self) -> &str {
        self.package_path.get_package_uuid()
    }
}

fn serialize_packages<S>(
    packages: &BTreeMap<Reverse<String>, PackageInstance>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(packages.len()))?;
    for value in packages.values() {
        seq.serialize_element(value)?;
    }
    seq.end()
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MoveConfig {
    // The key is the "uuid" defined in the Suibase.toml.

    // Last reported location of the .toml files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    // Sorted packages (most recently published first).
    #[serde(serialize_with = "serialize_packages")]
    packages: BTreeMap<Reverse<String>, PackageInstance>, // Key is timestamp.
}

impl MoveConfig {
    pub fn new() -> Self {
        Self {
            path: None,
            packages: BTreeMap::new(),
        }
    }

    pub fn contains(&self, package_timestamp: &str) -> bool {
        self.packages
            .contains_key(&Reverse(package_timestamp.to_string()))
    }

    fn delete_package_instance(&mut self, package_path: &PackagePath) -> bool {
        // Remove the package_instance from the map.
        self.packages
            .remove(&Reverse(package_path.get_package_timestamp().to_string()))
            .is_some()
    }

    fn add_package_instance(&mut self, package_instance: PackageInstance) -> bool {
        // Check if the package_instance is already in the map.
        if self.contains(&package_instance.ts) {
            return false;
        }

        // Insert the package_instance into the map.
        self.packages
            .insert(Reverse(package_instance.ts.to_string()), package_instance);
        true
    }
}

impl Default for MoveConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkdirPackagesResponse {
    pub header: Header,

    // One entry per distinct Move.toml published.
    //
    // Hashmap Key is a base32+md5sum of the "uuid" defined
    // in the Suibase.toml co-located with the Move.toml.
    //
    // For each MoveConfig, zero or more package instances
    // might have been published. MoveConfig keep track of
    // the latest instance.
    //
    // Among the move_configs, there is an additional constraint:
    //   - The MoveConfig.path must all be distinct.
    //
    move_configs: HashMap<String, MoveConfig>, // Key is the UUID
}

impl WorkdirPackagesResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            move_configs: HashMap::new(),
            //move_configs_set: HashSet::new(),
        }
    }

    pub fn contains(&self, package_path: &PackagePath) -> bool {
        if let Some(move_config) = self.move_configs.get(package_path.get_package_uuid()) {
            return move_config.contains(package_path.get_package_timestamp());
        }
        false
    }

    pub fn is_most_recent(&self, package_uuid: &str, package_timestamp: &str) -> bool {
        if let Some(move_config) = self.move_configs.get(package_uuid) {
            if let Some((timestamp, _)) = move_config.packages.iter().next() {
                return timestamp.0 == package_timestamp;
            }
        }
        false
    }

    pub fn package_count(&self) -> usize {
        self.move_configs
            .iter()
            .map(|(_, move_config)| move_config.packages.len())
            .sum()
    }

    // Create an iterator of all PackagePath in the WorkdirPackagesResponse.
    pub fn iter_package_paths(&self) -> impl Iterator<Item = &PackagePath> {
        self.move_configs.iter().flat_map(|(_, move_config)| {
            move_config
                .packages
                .iter()
                .map(|(_, package_instance)| package_instance.get_package_path())
        })
    }

    // Create an iterator of *most recent* PackageInstance for every UUID.
    pub fn iter_most_recent_package_instance(&self) -> impl Iterator<Item = &PackageInstance> {
        self.move_configs.iter().filter_map(|(_, move_config)| {
            move_config
                .packages
                .iter()
                .next()
                .map(|(_, package_instance)| package_instance)
        })
    }

    pub fn iter_mut_most_recent_package_instance(
        &mut self,
    ) -> impl Iterator<Item = &mut PackageInstance> {
        self.move_configs.iter_mut().filter_map(|(_, move_config)| {
            move_config
                .packages
                .iter_mut()
                .next()
                .map(|(_, package_instance)| package_instance)
        })
    }

    // Returns true if a change was performed.
    pub fn delete_package_instance(&mut self, package_path: &PackagePath) -> bool {
        if let Some(move_config) = self.move_configs.get_mut(package_path.get_package_uuid()) {
            return move_config.delete_package_instance(package_path);
        }
        false
    }

    // Returns true if a change was performed.
    pub fn add_package_instance(
        &mut self,
        package_instance: PackageInstance,
        move_toml_path: Option<String>,
    ) -> bool {
        if let Some(move_config) = self
            .move_configs
            .get_mut(package_instance.get_package_uuid())
        {
            return move_config.add_package_instance(package_instance);
        } else {
            // Delete any other move_configs element where path equals move_toml_path.
            if let Some(move_toml_path) = &move_toml_path {
                self.move_configs.retain(|_, config| {
                    if let Some(path) = &config.path {
                        if path == move_toml_path {
                            return false;
                        }
                    }
                    true
                });
            }

            // Create a new MoveConfig in move_configs for this UUID.
            let package_uuid = package_instance.get_package_uuid().to_string();
            let mut move_config = MoveConfig::new();
            move_config.path = move_toml_path;
            move_config.add_package_instance(package_instance);
            self.move_configs.insert(package_uuid, move_config);
            true
        }
    }

    // Follow-up with calling this after all changes with add/delete_package_instance() are done.
    /*
    pub fn update_move_configs_set(&mut self) -> &HashSet<PackagePath> {
        // Costly operation, do only when move_configs was changed.
        self.move_configs_set = self
            .move_configs
            .iter()
            .flat_map(|(uuid, move_config)| {
                let mut packages = Vec::new();
                for older_package in &move_config.packages {
                    packages.push(PackagePath::new(
                        older_package.package_name.clone(),
                        uuid.clone(),
                        older_package.package_timestamp.clone(),
                    ));
                }
                packages
            })
            .collect();

        &self.move_configs_set
    }*/
}

impl Default for WorkdirPackagesResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionedEq for WorkdirPackagesResponse {
    fn versioned_eq(&self, other: &Self) -> bool {
        // Purposely do not include header in the comparison.
        self.move_configs == other.move_configs
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionsResponse {
    pub header: Header,
    pub versions: Vec<Header>,

    pub asui_selection: Option<String>, // Last confirmed applied asui.
}

impl VersionsResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            versions: Vec::new(),
            asui_selection: None,
        }
    }
}

impl Default for VersionsResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionedEq for VersionsResponse {
    fn versioned_eq(&self, other: &Self) -> bool {
        // Purposely do not include header in the comparison.
        self.versions == other.versions && self.asui_selection == other.asui_selection
    }
}

#[rpc(server)]
pub trait ProxyApi {
    /// Returns data about all the RPC/Websocket links
    /// for a given workdir.
    ///
    /// By default fetch everything, but can reduce load
    /// with the options.
    #[method(name = "getLinks")]
    async fn get_links(
        &self,
        workdir: String,
        summary: Option<bool>,
        links: Option<bool>,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
    ) -> RpcResult<LinksResponse>;

    #[method(name = "fsChange")]
    async fn fs_change(&self, path: String) -> RpcResult<InfoResponse>;
    
    /// Reset all server statistics for a workdir
    /// This is primarily for testing purposes
    #[method(name = "resetServerStats")]
    async fn reset_server_stats(
        &self,
        workdir: String,
    ) -> RpcResult<SuccessResponse>;
}

#[rpc(server)]
pub trait GeneralApi {
    // Get versions of all available "group of" data.
    //
    // Can be used by caller to detect changes by polling.
    //
    // When detecting a change, it is often followed by
    // another fetch to get the latest (e.g. get_workdir_status()).
    //
    #[method(name = "getVersions")]
    async fn get_versions(&self, workdir: Option<String>) -> RpcResult<VersionsResponse>;

    #[method(name = "workdirCommand")]
    async fn workdir_command(&self, workdir: String, command: String)
        -> RpcResult<SuccessResponse>;

    // Get status of a specific workdir.
    //
    // Can optionally request a specific response version and it will
    // be returned if it is available (only latest is available).
    //
    // Will return an error if requesting with outdated/invalid UUIDs.
    // You can get the latest UUIDs with getVersions().
    #[method(name = "getWorkdirStatus")]
    async fn get_workdir_status(
        &self,
        workdir: String,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<WorkdirStatusResponse>;

    // Allow to modify the asui selection.
    //
    // Choices are "localnet", "devnet", "testnet" or "mainnet".
    //
    // Returns success only after confirmed applied to Suibase (retry may have occurred).
    #[method(name = "setAsuiSelection")]
    async fn set_asui_selection(&self, workdir: String) -> RpcResult<SuccessResponse>;

    // Notify the daemon to update its status for a specific workdir.
    //
    // Should be called only when a CLI command was executed and is known
    // to have modified "something".
    //
    // The backend periodically refresh all its status. This notification
    // just trig an "immediate" refresh.
    #[method(name = "workdirRefresh")]
    async fn workdir_refresh(&self, workdir: String) -> RpcResult<SuccessResponse>;
}

#[rpc(server)]
pub trait PackagesApi {
    #[method(name = "getWorkdirEvents")]
    async fn get_workdir_events(
        &self,
        workdir: String,
        after_ts: Option<String>,
        last_ts: Option<String>,
    ) -> RpcResult<WorkdirSuiEventsResponse>;

    #[method(name = "getWorkdirPackages")]
    async fn get_workdir_packages(
        &self,
        workdir: String,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<WorkdirPackagesResponse>;

    #[method(name = "prePublish")]
    async fn pre_publish(
        &self,
        workdir: String,
        move_toml_path: String,
        package_name: String,
    ) -> RpcResult<SuccessResponse>;

    #[method(name = "postPublish")]
    async fn post_publish(
        &self,
        workdir: String,
        move_toml_path: String,
        package_name: String,
        package_uuid: String,
        package_timestamp: String,
        package_id: String,
    ) -> RpcResult<SuccessResponse>;
}

#[rpc(server)]
pub trait MockApi {
    /// Control mock server behavior for testing
    #[method(name = "mockServerControl")]
    async fn mock_server_control(
        &self,
        alias: String,
        behavior: crate::shared_types::MockServerBehavior,
    ) -> RpcResult<SuccessResponse>;

    /// Get detailed statistics for a mock server (read-only)
    #[method(name = "mockServerStats")]
    async fn mock_server_stats(
        &self,
        alias: String,
    ) -> RpcResult<crate::shared_types::MockServerStatsResponse>;

    /// Reset statistics for a mock server
    #[method(name = "mockServerReset")]
    async fn mock_server_reset(
        &self,
        alias: String,
    ) -> RpcResult<SuccessResponse>;

}
