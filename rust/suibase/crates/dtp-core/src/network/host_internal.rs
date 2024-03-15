// What is the type naming convention?
//
// "Object"         --> Name of the object in the move package
//
// "ObjectInternal" --> Local memory representation, may have additional
//                      fields not found on the network.
//
// "ObjectMoveRaw" --> Serialized fields as intended to be for the network
//                     *MUST* match the Move Sui definition of a given version.
//
// Example:
//   "Host"
//   "HostInternal"
//   "HostMoveRaw"
//

use log::info;
use serde::Deserialize;
use sui_sdk::types::base_types::{ObjectID, SuiAddress};
use sui_types::id::UID;

use crate::types::SuiSDKParamsRPC;

#[derive(Deserialize, Debug)]
pub struct WeakRef {
    // Refer to a Sui object, but can't assume it still exists (e.g. was deleted).
    //
    // Flags mapping
    //   Lowest 2 bits are reserved for weak reference management:
    //
    //     Bit1  Bit0
    //     ==========
    //       0    0   Reference is not initialized
    //       0    1   Reference was initialized, but object is last known to not exist anymore.
    //       1    0   Reference is considered valid and object is last known to exist.
    //       1    1   Reserved for future
    //
    //   The highest 6 bits [Bit8..Bit3] can mean anything the user wants.
    //   See set_flags_user() and get_flags_user().
    //
    // Reference is an address, which can easily be converted from/to Object ID.
    flags: u8,
    reference: SuiAddress,
}

#[derive(Deserialize, Debug)]
pub struct Connection {
    tc: WeakRef, // Reference on the TransportControl (for slow discovery).
}

#[derive(Deserialize, Debug)]
pub struct ConnAcceptedStats {
    conn_accepted: u64,     // Normally accepted connection.
    conn_accepted_lru: u64, // Accepted after LRU eviction of another connection.
}

#[derive(Deserialize, Debug)]
pub struct ConnClosedStats {
    conn_closed_srv: u64,          // Successful close initiated by server.
    conn_closed_cli: u64,          // Successful close initiated by client.
    conn_closed_exp: u64, // Normal expiration initiated by protocol (e.g. Ping Connection iddle).
    conn_closed_lru: u64, // Close initiated by least-recently-used (LRU) algo when cons limit reach.
    conn_closed_srv_sync_err: u64, // Server caused a sync protocol error.
    conn_closed_clt_sync_err: u64, // Client caused a sync protocol error.
}

#[derive(Deserialize, Debug)]
pub struct ConnRejectedStats {
    conn_rej_host_max_con: u64, // Max Host connection limit reached.
    conn_rej_srv_max_con: u64,  // Max Service connection limit reached.
    conn_rej_firewall: u64,     // Firewall rejected. TODO more granular reasons.
    conn_rej_srv_down: u64,     // Connection requested while server is down.
    conn_rej_cli_err: u64,      // Error in client request.
    conn_rej_cli_no_fund: u64,  // Client not respecting funding SLA.
}
#[derive(Deserialize, Debug)]
pub struct Service {
    service_idx: u8,
    fee_per_request: u64,
    conn_accepted: ConnAcceptedStats,
    conn_rejected: ConnRejectedStats,
    conn_closed: ConnClosedStats,
}

#[derive(Deserialize, Debug)]
pub struct HostConfig {
    max_con: u32,
}

// Data structure that **must** match the Move Host object
#[derive(Deserialize, Debug)]
pub struct HostMoveRaw {
    id: UID,
    authority: SuiAddress,
    config: HostConfig,
    services: Vec<Service>,
}

#[derive(Debug)]
pub struct HostInternal {
    pub(crate) object_id: ObjectID,
    pub(crate) authority: Option<SuiAddress>,
    pub(crate) raw: Option<HostMoveRaw>, // Data from network (as-is)
}

pub(crate) async fn get_host_internal_by_id(
    rpc: &SuiSDKParamsRPC,
    host_object_id: ObjectID,
) -> Result<Option<HostInternal>, anyhow::Error> {
    info!("get_host_internal_by_id 1");
    let raw = super::common_rpc::fetch_raw_move_object::<HostMoveRaw>(rpc, host_object_id).await?;
    info!("get_host_internal_by_id 2");
    if raw.is_none() {
        info!("get_host_internal_by_id 3");
        return Ok(None);
    }
    let raw = raw.unwrap();

    // Map the Host Move object into the local memory representation.
    let ret = HostInternal {
        object_id: host_object_id.clone(),
        authority: Some(raw.authority),
        raw: Some(raw),
    };
    Ok(Some(ret))
}

pub(crate) async fn get_host_internal_by_auth(
    rpc: &SuiSDKParamsRPC,
    package_id: &ObjectID,
    address: &SuiAddress,
) -> Result<Option<HostInternal>, anyhow::Error> {
    // When returning Ok(None) it means that it was verified
    // that this address does not OWN a Host object.
    info!("get_host_internal_by_auth 1");
    let raw = super::common_rpc::fetch_raw_move_object_by_auth::<HostMoveRaw>(
        rpc, package_id, "host", "Host", address,
    )
    .await?;
    info!("get_host_internal_by_auth 2");
    if raw.is_none() {
        info!("get_host_internal_by_auth 3");
        return Ok(None);
    }
    let raw = raw.unwrap();

    // Map the Host Move object into the local memory representation.
    let ret = HostInternal {
        object_id: *raw.id.object_id(),
        authority: Some(raw.authority),
        raw: Some(raw),
    };
    Ok(Some(ret))
}

impl HostInternal {
    pub(crate) fn new(object_id: ObjectID) -> HostInternal {
        HostInternal {
            object_id,
            authority: None,
            raw: None,
        }
    }

    pub fn object_id(&self) -> ObjectID {
        self.object_id
    }

    pub fn authority(&self) -> Option<SuiAddress> {
        self.authority
    }
}
