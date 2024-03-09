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

use crate::types::{DTPError, SuiSDKParamsRPC};

use serde::Deserialize;
use sui_json_rpc_types::{SuiData, SuiObjectDataOptions};
use sui_sdk::types::base_types::{ObjectID, SuiAddress};
use sui_types::id::UID;

// Data structure that **must** match the Move Host object
#[derive(Deserialize, Debug)]
pub struct HostMoveRaw {
    id: UID,
    flgs: u8,
    adm: SuiAddress,
    conn_req: u64,
    conn_sdd: u64,
    conn_del: u64,
    conn_rcy: u64,
    max_con: u16,
}

#[derive(Debug)]
pub struct HostInternal {
    pub(crate) object_id: ObjectID,
    pub(crate) admin_address: Option<SuiAddress>,
    pub(crate) raw: Option<HostMoveRaw>, // Data from network (as-is)
}

pub(crate) async fn fetch_host_move_object(
    rpc: &SuiSDKParamsRPC,
    host_object_id: ObjectID,
) -> Result<HostMoveRaw, anyhow::Error> {
    // TODO Revisit for robustness
    let sui_client = rpc.sui_client.as_ref().expect("Could not create SuiClient");

    let resp = sui_client
        .inner
        .read_api()
        .get_object_with_options(host_object_id, SuiObjectDataOptions::default().with_bcs())
        .await?
        .into_object();

    if let Err(e) = resp {
        return Err(DTPError::DTPFailedFetchObject {
            object_type: "Host".to_string(),
            object_id: host_object_id.to_string(),
            inner: e.to_string(),
        }
        .into());
    }

    // Deserialize the BCS data into a HostMoveRaw
    let resp = resp.unwrap();
    let raw_data = resp.to_string(); // Copy to string for debug purpose... optimize this later?
    let sui_raw_data = resp.bcs;
    if let Some(sui_raw_data) = sui_raw_data {
        if let Some(sui_raw_mov_obj) = sui_raw_data.try_into_move() {
            return sui_raw_mov_obj.deserialize();
        }
    };

    Err(DTPError::DTPFailedConvertBCS {
        object_type: "Host".to_string(),
        object_id: host_object_id.to_string(),
        raw_data,
    }
    .into())
}

pub(crate) async fn get_host_by_id(
    rpc: &SuiSDKParamsRPC,
    host_object_id: ObjectID,
) -> Result<HostInternal, anyhow::Error> {
    let raw = fetch_host_move_object(rpc, host_object_id).await?;

    // Map the Host Move object into the local memory representation.
    let ret = HostInternal {
        object_id: host_object_id,
        admin_address: Some(raw.adm),
        raw: Some(raw),
    };
    Ok(ret)
}

impl HostInternal {
    pub(crate) fn new(object_id: ObjectID) -> HostInternal {
        HostInternal {
            object_id,
            admin_address: None,
            raw: None,
        }
    }

    pub fn object_id(&self) -> ObjectID {
        self.object_id
    }

    pub fn admin_address(&self) -> Option<SuiAddress> {
        self.admin_address
    }
}
