// Defines the JSON-RPC API.
//
// Intended Design (WIP)
//
// The API defined here is registered and served by jsonrpsee  (See api_server.rs).
//
// The definitions here are just thin layers and most of the heavy lifting is done
// in other modules.
//
// When doing a request that can "mutate" the process (other than API statistics), the request handler
// emit a message toward the AdminController describing the action needed. The AdminController perform the
// modification and provides the response with a returning tokio OneShot channel.
//
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
    //#[schemars(with = "u32")]
    //#[serde_as(as = "u32")]
    pub status: String, // This is the combined "Multi-Link status". One of "OK", "DOWN", "DISABLED", "DEGRADED"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<LinksSummary>,

    // List of links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<LinkStats>>,

    // This is the output when the option 'display' is true.
    // Will also change the default to false for the summary/links/display output.
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
            status: "DISABLED".to_string(),
            summary: None,
            links: None,
            display: None,
            debug: None,
        }
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
}
