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
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LinkStats {
    pub alias: String,

    // -100.0 to +100.0 (sign and decimal always included)
    pub health_pct: String,

    //    0 or 0.0 to 100.0
    //
    // Recommend display as-is, two exceptions:
    //      0 is a true zero (e.g. display as "0.0")
    //    0.0 should be displayed as "<0.1"
    pub load_pct: String,

    //  0.00 to 999.99 (decimal always included)
    //  999.99 should be displayed as ">1 secs"
    pub resp_time: String,

    // 0 or 0.000 to 100.0
    //
    // Recommend display as-is, two exceptions:
    //      0 is a true zero (e.g. display as "0")
    //  0.000 should be displayed as "<0.001"
    pub retry_pct: String,
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
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinksSummary {
    // Each request counted only once, even when retried.
    pub success_on_first_attempt: u32,
    pub success_on_retry: u32,
    pub fail_retry: u32,
    pub fail_network_down: u32,
    pub fail_bad_request: u32,

    // Same variables but for percent (and as string)
    pub success_on_first_attempt_pct: String,
    pub success_on_retry_pct: String,
    pub fail_retry_pct: String,
    pub fail_network_down_pct: String,
    pub fail_bad_request_pct: String,
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinksResponse {
    //#[schemars(with = "u32")]
    //#[serde_as(as = "u32")]
    pub proxy_enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<LinksSummary>,

    // List of links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<LinkStats>>,
}

impl LinksResponse {
    pub fn new() -> Self {
        Self {
            proxy_enabled: false,
            summary: None,
            links: None,
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
    async fn get_links(&self, workdir: String) -> RpcResult<LinksResponse>;
}
