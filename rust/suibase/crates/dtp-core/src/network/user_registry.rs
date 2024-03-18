// What is the type naming convention?
//
// "Object"         --> Name of the object in the move package
//
// "ObjectInternal" --> Local memory representation, may have additional
//                      fields not found on the network.
//
// "ObjectMoveRaw" --> Serialized fields as intended to be for the network
//                 *MUST* match the Move Sui definition of a given version.
//
// Example:
//   "UserRegistry"
//   "UserRegistryInternal"
//   "UserRegistryMoveRaw"
//

use log::info;

use sui_sdk::{
    json::SuiJsonValue,
    types::base_types::{ObjectID, SuiAddress},
};

use super::UserRegistryMoveRaw;
use crate::types::{DTPError, SuiSDKParamsRPC, SuiSDKParamsTxn};

// Data structure that **must** match the Move Host object

#[derive(Debug)]
pub struct UserRegistryInternal {
    object_id: ObjectID,
    localhost_id: Option<ObjectID>,
    is_dirty: bool, // Dirty means a change was done and an update to the network is needed.
    raw: Option<UserRegistryMoveRaw>, // Data read/expected on network.
}

// Create an internal representation by consuming the raw Move object.
fn raw_to_internal(raw: UserRegistryMoveRaw) -> Result<UserRegistryInternal, anyhow::Error> {
    // Convert raw.host_addr to an ObjectID.
    let result = ObjectID::from_bytes(raw.host_addr);
    if let Err(e) = result {
        let desc = format!("host_addr ObjectIDParseError={}", e);
        return Err(DTPError::DTPFailedRegistryLoad { desc }.into());
    }
    let localhost_id = Some(result.unwrap());

    let ret = UserRegistryInternal {
        object_id: *raw.id.object_id(),
        localhost_id,
        is_dirty: false,
        raw: Some(raw),
    };

    Ok(ret)
}

pub(crate) async fn get_user_registry_internal_by_id(
    rpc: &SuiSDKParamsRPC,
    object_id: ObjectID,
) -> Result<Option<UserRegistryInternal>, anyhow::Error> {
    info!("get_user_registry_internal_by_id 1");
    let raw =
        super::common_rpc::fetch_raw_move_object::<UserRegistryMoveRaw>(rpc, object_id).await?;
    info!("get_user_registry_internal_by_id 2");
    if raw.is_none() {
        info!("get_user_registry_internal_by_id 3");
        return Ok(None);
    }
    let ret = raw_to_internal(raw.unwrap())?;
    Ok(Some(ret))
}

pub(crate) async fn get_user_registry_internal_by_auth(
    rpc: &SuiSDKParamsRPC,
    package_id: &ObjectID,
    address: &SuiAddress,
) -> Result<Option<UserRegistryInternal>, anyhow::Error> {
    // When returning Ok(None) it means that it was verified
    // that this address does not OWN a UserRegistry.
    info!("get_user_registry_internal_by_auth 1");
    let raw = super::common_rpc::fetch_raw_move_object_by_auth::<UserRegistryMoveRaw>(
        rpc,
        package_id,
        "user_registry",
        "UserRegistry",
        address,
    )
    .await?;
    info!("get_user_registry_internal_by_auth 2");
    if raw.is_none() {
        info!("get_user_registry_internal_by_auth 3");
        return Ok(None);
    }
    let ret = raw_to_internal(raw.unwrap())?;
    Ok(Some(ret))
}

pub(crate) async fn create_registry_on_network(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    localhost_id: ObjectID,
) -> Result<UserRegistryInternal, anyhow::Error> {
    // There should be only one UserRegistry per client address.
    //
    // Caller is responsible to verify if one already exists.

    //let vargs: Vec<u8> = vec![];
    let call_args = vec![
        SuiJsonValue::from_object_id(localhost_id),
        //SuiJsonValue::from_bcs_bytes(None, &vargs).unwrap(),
    ];
    let new_object_id = super::common_rpc::do_move_call_ret_id(
        rpc,
        txn,
        "user_registry",
        "new_and_transfer", // Fix syntax
        "user_registry",
        "UserRegistry",
        call_args,
    )
    .await?;

    // Success.
    Ok(UserRegistryInternal {
        object_id: new_object_id,
        localhost_id: Some(localhost_id),
        is_dirty: false,
        raw: None,
    })
}

impl UserRegistryInternal {
    pub(crate) fn new(object_id: ObjectID) -> UserRegistryInternal {
        UserRegistryInternal {
            object_id,
            localhost_id: None,
            is_dirty: false,
            raw: None,
        }
    }

    pub fn object_id(&self) -> ObjectID {
        self.object_id
    }

    pub fn localhost_id(&self) -> Option<ObjectID> {
        self.localhost_id
    }

    pub fn set_localhost_id(&mut self, new_localhost_id: Option<ObjectID>) {
        if new_localhost_id != self.localhost_id {
            self.is_dirty = true;
            self.localhost_id = new_localhost_id;
        }
    }
}
