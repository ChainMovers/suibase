use std::collections::HashMap;

use common::workers::PackageTrackingState;

// Defines the JSON-RPC API.
//
// Design:
//
// The API defined here is registered and served by jsonrpsee  (See api_server.rs).
//
// This is a thin layer and most of the heavy lifting is done in other modules.
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
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Clone, Default, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkStats {
    // The alias of the link, as specified in the config file.
    pub alias: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub status: String, // Empty string, "OK" or "DOWN"

    #[serde(skip_serializing_if = "String::is_empty")]
    pub health_pct: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub load_pct: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub resp_time: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub success_pct: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub error_info: String, // Sometime more info when DOWN.
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

// General purpose RPC response.
//
// The interpretation depends on the method.
#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub header: Header,
    pub info: String, // "Success" or info on failure.

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>, // Human friendly representation.

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>, // JSON formatted representation.

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>, // Additional JSON debug information.
}

impl InfoResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            info: "Unknown Error".to_string(),
            display: None,
            data: None,
            debug: None,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {
    // On success:
    //   'result' is "Success".
    // On error:
    //  'result' is a human-readable error message.
    //  'bytes', 'sender', 'rtt' and 'seq' are empty strings.
    pub header: Header,
    pub bytes: String,  // Total bytes received.
    pub sender: String, // Hex Host address of the responder (starts with 0x).
    pub seq: String,    // Sequence number. Helps to diagnose packet loss.
    pub rtt: String,    // Round-trip time in microseconds.
    pub result: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>, // Human friendly representation.

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>, // JSON formatted representation.

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>, // Additional JSON debug information.
}

impl PingResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            bytes: "".to_string(),
            sender: "".to_string(),
            seq: "".to_string(),
            rtt: "".to_string(),
            result: "Unknown Error".to_string(),
            display: None,
            data: None,
            debug: None,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusService {
    pub label: String, // "localnet process", "proxy server", "multi-link RPC" etc...
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub asui_selection: Option<String>,

    // Finer grain status for each process/feature/service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<StatusService>>,

    // This is the output when the option 'display' is true.
    // Will also change the default to false for all the other fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    // This is the output when the option 'debug' is true.
    // Will also change the default to true for the other fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,
}

impl WorkdirStatusResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            status: None,
            status_info: None,
            client_version: None,
            network_version: None,
            asui_selection: None,
            services: None,
            display: None,
            debug: None,
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
            && self.asui_selection == other.asui_selection
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
pub struct SuiObjectInstance {
    object_id: String,
}

impl SuiObjectInstance {
    pub fn new(object_id: String) -> Self {
        Self { object_id }
    }
    pub fn object_id(&self) -> &str {
        &self.object_id
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageInstance {
    pub package_id: String, // Hexa (no 0x).
    pub package_name: String,
    pub package_timestamp: String,
    pub init_objects: Option<Vec<SuiObjectInstance>>,
}

impl PackageInstance {
    pub fn new(package_id: String, package_name: String, package_timestamp: String) -> Self {
        Self {
            package_id,
            package_name,
            package_timestamp,
            init_objects: None,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MoveConfig {
    // The key is the "uuid" defined in the Suibase.toml.

    // Last reported location of the .toml files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    // Last publish instance of the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_package: Option<PackageInstance>,

    // Packages previously published (does not include the current).
    // Useful for tracking older package id for debug browsing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub older_packages: Vec<PackageInstance>,

    pub tracking_state: u32, // Helpful for debugging.
}

impl MoveConfig {
    pub fn new() -> Self {
        Self {
            path: None,
            latest_package: None,
            older_packages: Vec::new(),
            tracking_state: PackageTrackingState::new().into(),
        }
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
pub struct PackagesConfigResponse {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_configs: Option<HashMap<String, MoveConfig>>,

    // This is the output when the option 'display' is true.
    // Will also change the default to false for all the other fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    // This is the output when the option 'debug' is true.
    // Will also change the default to true for the other fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,
}

impl PackagesConfigResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            move_configs: None,
            display: None,
            debug: None,
        }
    }
}

impl Default for PackagesConfigResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionedEq for PackagesConfigResponse {
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
}

impl VersionsResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            versions: Vec::new(),
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
        self.versions == other.versions
    }
}

#[rpc(server)]
pub trait DtpApi {
    #[method(name = "fsChange")]
    async fn fs_change(&self, path: String) -> RpcResult<InfoResponse>;

    #[method(name = "ping")]
    async fn ping(
        &self,
        workdir: String,
        host_addr: String,
        bytes: Option<String>,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
    ) -> RpcResult<PingResponse>;

    #[method(name = "publish")]
    async fn publish(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
    ) -> RpcResult<InfoResponse>;
}

#[rpc(server)]
pub trait GeneralApi {
    // Get version of latest for everything.
    // (e.g. WorkdirStatusResponse, WorkdirEventsResponse etc...)
    //
    // Can be used by caller to detect changes by polling.
    //
    // Output is always JSON.
    #[method(name = "getVersions")]
    async fn get_versions(&self, workdir: String) -> RpcResult<VersionsResponse>;

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

    // Get information for a specific workdir.
    // By default, just a few fields for user-level operation of DTP.
    // "debug" will provide extensive details (most globals).
    #[method(name = "info")]
    async fn info(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
    ) -> RpcResult<InfoResponse>;
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

    #[method(name = "getWorkdirPackagesConfig")]
    async fn get_workdir_packages_config(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<PackagesConfigResponse>;

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
