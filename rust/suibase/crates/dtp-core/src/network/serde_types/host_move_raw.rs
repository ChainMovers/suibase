// Must match Move object definition(s) on network

use serde::Deserialize;

use sui_sdk::types::base_types::SuiAddress;
use sui_types::id::UID;

#[derive(Deserialize, Debug)]
pub struct ConnAcceptedStats {
    pub conn_accepted: u64,     // Normally accepted connection.
    pub conn_accepted_lru: u64, // Accepted after LRU eviction of another connection.
}

#[derive(Deserialize, Debug)]
pub struct ConnClosedStats {
    pub conn_closed_srv: u64,          // Successful close initiated by server.
    pub conn_closed_cli: u64,          // Successful close initiated by client.
    pub conn_closed_exp: u64, // Normal expiration initiated by protocol (e.g. Ping Connection iddle).
    pub conn_closed_lru: u64, // Close initiated by least-recently-used (LRU) algo when cons limit reach.
    pub conn_closed_srv_sync_err: u64, // Server caused a sync protocol error.
    pub conn_closed_clt_sync_err: u64, // Client caused a sync protocol error.
}

#[derive(Deserialize, Debug)]
pub struct ConnRejectedStats {
    pub conn_rej_host_max_con: u64, // Max Host connection limit reached.
    pub conn_rej_srv_max_con: u64,  // Max Service connection limit reached.
    pub conn_rej_firewall: u64,     // Firewall rejected. TODO more granular reasons.
    pub conn_rej_srv_down: u64,     // Connection requested while server is down.
    pub conn_rej_cli_err: u64,      // Error in client request.
    pub conn_rej_cli_no_fund: u64,  // Client not respecting funding SLA.
}
#[derive(Deserialize, Debug)]
pub struct Service {
    pub service_idx: u8,
    pub fee_per_request: u64,
    pub conn_accepted: ConnAcceptedStats,
    pub conn_rejected: ConnRejectedStats,
    pub conn_closed: ConnClosedStats,
}

#[derive(Deserialize, Debug)]
pub struct HostConfig {
    pub max_con: u32,
}

// Data structure that **must** match the Move Host object
#[derive(Deserialize, Debug)]
pub struct HostMoveRaw {
    pub id: UID,
    pub authority: SuiAddress,
    pub config: HostConfig,
    pub services: Vec<Service>,
}
