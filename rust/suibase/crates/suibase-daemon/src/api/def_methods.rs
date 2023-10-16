use hyper::header;
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
use super::def_header::Header;
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
pub struct StatusResponse {
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

impl StatusResponse {
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

impl Default for StatusResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuiEvents {
    pub message: String,
    pub timestamp: String,
}

impl SuiEvents {
    pub fn new(label: String) -> Self {
        Self {
            message: label,
            timestamp: "".to_string(),
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuiEventsResponse {
    pub header: Header,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<SuiEvents>>,
}

impl SuiEventsResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            events: None,
        }
    }
}

impl Default for SuiEventsResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub header: Header,
    pub success: bool,
}

impl SuccessResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            success: false,
        }
    }
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self::new()
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
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleConfig {
    pub name: Option<String>, // "localnet process", "proxy server", "multi-link RPC" etc...
    pub id: Option<String>,   // OK, DOWN, DEGRADED
}

impl ModuleConfig {
    pub fn new() -> Self {
        Self {
            name: None,
            id: None,
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModulesConfigResponse {
    pub header: Header,

    // Last publish instance of each module.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<ModuleConfig>>,

    // This is the output when the option 'display' is true.
    // Will also change the default to false for all the other fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    // This is the output when the option 'debug' is true.
    // Will also change the default to true for the other fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,
}

impl ModulesConfigResponse {
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            modules: None,
            display: None,
            debug: None,
        }
    }
}

impl Default for ModulesConfigResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[rpc(server)]
pub trait GeneralApi {
    #[method(name = "getStatus")]
    async fn get_status(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<StatusResponse>;
}

#[rpc(server)]
pub trait ModulesApi {
    #[method(name = "getEvents")]
    async fn get_events(
        &self,
        workdir: String,
        after_ts: Option<String>,
        last_ts: Option<String>,
    ) -> RpcResult<SuiEventsResponse>;

    #[method(name = "getModulesConfig")]
    async fn get_modules_config(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<ModulesConfigResponse>;

    #[method(name = "publish")]
    async fn publish(
        &self,
        workdir: String,
        module_name: String,
        module_id: String,
    ) -> RpcResult<SuccessResponse>;
}
