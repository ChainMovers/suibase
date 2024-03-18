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
use sui_sdk::types::base_types::{ObjectID, SuiAddress};

use crate::types::SuiSDKParamsRPC;

use super::HostMoveRaw;

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
        object_id: host_object_id,
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
