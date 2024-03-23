// Utility function to do common RPC calls.

use std::str::FromStr;

use anyhow::bail;
use log::info;
use move_core_types::language_storage::StructTag;
use serde::Deserialize;
use shared_crypto::intent::Intent;
use sui_json_rpc_types::{
    SuiData, SuiObjectDataFilter, SuiObjectDataOptions, SuiObjectResponse, SuiObjectResponseQuery,
    SuiTransactionBlockResponse, SuiTransactionBlockResponseOptions,
};
use sui_keys::keystore::AccountKeystore;
use sui_sdk::json::SuiJsonValue;
use sui_types::base_types::SuiAddress;
use sui_types::{
    base_types::ObjectID, quorum_driver_types::ExecuteTransactionRequestType,
    transaction::Transaction,
};

use sui_types::error::SuiObjectResponseError;

use crate::types::{DTPError, SuiSDKParamsRPC, SuiSDKParamsTxn};
use serde::de::DeserializeOwned;

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

// Perform a mostly generic move call.
// Caller specify 'options' effects and deserialize the response.
pub(crate) async fn do_move_call(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    call_module: &str,            // e.g. api
    function: &str,               // e.g. create
    call_args: Vec<SuiJsonValue>, // Can be empty vec![]
    options: SuiTransactionBlockResponseOptions,
) -> Result<SuiTransactionBlockResponse, anyhow::Error> {
    let sui_client = match rpc.sui_client.as_ref() {
        Some(x) => &x.inner,
        None => bail!(DTPError::DTPMissingSuiClient),
    };
    let keystore = &txn.keystore.inner;

    let call_desc = format!(
        "{}::{}::{}({:?}) with signer {}",
        txn.package_id, call_module, function, call_args, rpc.client_address,
    );

    let move_call = sui_client
        .transaction_builder()
        .move_call(
            rpc.client_address,
            txn.package_id,
            call_module,
            function,
            vec![],
            call_args,
            None, // The node will pick a gas object from the signer.
            1000000000,
        )
        .await;
    if let Err(e) = move_call {
        return Err(DTPError::DTPFailedMoveCall {
            desc: format!("move_call failed for {}", call_desc),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
            inner: e.to_string(),
        }
        .into());
    }
    let move_call = move_call.unwrap();

    // Sign transaction.
    let signature =
        keystore.sign_secure(&rpc.client_address, &move_call, Intent::sui_transaction())?;

    let tx = Transaction::from_data(move_call, vec![signature]);
    let response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            tx,
            options,
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await;
    if response.is_err() {
        return Err(DTPError::DTPFailedMoveCall {
            desc: format!("response failed for {}", call_desc),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
            inner: format!("Inner error [{}]", response.unwrap_err()),
        }
        .into());
    }
    let response = response.unwrap();

    if !response.errors.is_empty() {
        let mut error_message = "Inner error [".to_string();
        for error in response.errors {
            error_message.push_str(&error.to_string());
        }
        error_message.push(']');

        bail!(DTPError::DTPFailedMoveCall {
            desc: format!("response errors for {}", call_desc),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
            inner: error_message
        });
    }
    Ok(response)
}

pub(crate) async fn do_move_call_no_ret(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    call_module: &str,            // e.g. api
    function: &str,               // e.g. open_connection
    call_args: Vec<SuiJsonValue>, // Can be empty vec![]
) -> Result<(), anyhow::Error> {
    let options = SuiTransactionBlockResponseOptions::new();
    let response = do_move_call(rpc, txn, call_module, function, call_args, options).await;
    if let Err(e) = response {
        // TODO Append event_type info to error.
        return Err(e);
    }
    let _ = response.unwrap();
    Ok(())
}

// Function that perform a move call and deserialize an expected single event 'T' effect.
// Returns Ok(None) if the call succeed, but the event was not emitted.
pub(crate) async fn do_move_call_ret_event<T>(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    call_module: &str,            // e.g. api
    function: &str,               // e.g. open_connection
    event_module: &str,           // e.g. events
    event_type: &str,             // e.g. ConnReq
    call_args: Vec<SuiJsonValue>, // Can be empty vec![]
) -> Result<T, anyhow::Error>
where
    T: DeserializeOwned,
{
    let options = SuiTransactionBlockResponseOptions::new()
        .with_events()
        .with_effects();
    let response = do_move_call(rpc, txn, call_module, function, call_args, options).await;
    if let Err(e) = response {
        // TODO Append event_type info to error.
        return Err(e);
    }
    let response = response.unwrap();

    // Get the expected event effect.
    let events = response.events.unwrap();

    // Iterate the object changes, look for the "event_module::event_type".

    // TODO Optimize this?
    let tag_str = format!("{}::{}::{}", txn.package_id, event_module, event_type);
    let tag = StructTag::from_str(&tag_str)?;

    for event in events.data {
        info!("event {:?}", event);
        if event.package_id == txn.package_id && event.type_ == tag {
            // BCS deserialization.
            let event_obj = bcs::from_bytes::<T>(&event.bcs);
            if let Err(e) = event_obj {
                bail!(DTPError::DTPFailedConvertBCS {
                    object_type: std::any::type_name::<T>().to_string(),
                    object_id: "NA".to_string(),
                    raw_data: format!("event[{:?} inner error[{}]", event, e),
                });
            }
            let event_obj = event_obj.unwrap();
            return Ok(event_obj);
        }
    }

    bail!(DTPError::DTPFailedMoveCall {
        desc: format!(
            "event {}:{} not found in response",
            event_module, event_type
        ),
        package_id: txn.package_id.to_string(),
        client_address: rpc.client_address.to_string(),
        inner: "".to_string()
    });
}

// A move call that returns the ID of a new object created.
pub(crate) async fn do_move_call_ret_id(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    call_module: &str,            // e.g. api
    function: &str,               // e.g. create
    new_object_module: &str,      // e.g. host
    new_object_type: &str,        // e.g. Host
    call_args: Vec<SuiJsonValue>, // Can be empty vec![]
) -> Result<ObjectID, anyhow::Error> {
    let options = SuiTransactionBlockResponseOptions::new()
        .with_object_changes()
        .with_effects();
    let response = do_move_call(rpc, txn, call_module, function, call_args, options).await;
    if let Err(e) = response {
        return Err(e);
    }
    let response = response.unwrap();

    // Get the id from the newly created Sui object.
    let object_changes = response.object_changes.unwrap();

    // Iterate the object changes, look for the needed object (e.g. "host::Host")
    let mut created_object_id = Option::<ObjectID>::None;
    for object_change in object_changes {
        info!("iter object {:?}", object_change);
        match object_change {
            sui_json_rpc_types::ObjectChange::Created {
                object_type,
                object_id,
                ..
            } => {
                // Check if the object_type is "host::Host"
                if object_type.name.to_string() == new_object_type
                    && object_type.module.to_string() == new_object_module
                {
                    created_object_id = Some(object_id)
                }
            }
            _ => {}
        }
    }
    if created_object_id.is_none() {
        bail!(DTPError::DTPFailedMoveCall {
            desc: format!(
                "object {}:{} not found in response",
                new_object_module, new_object_type
            ),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
            inner: "".to_string()
        });
    }

    // Success.
    Ok(created_object_id.unwrap())
}

pub(crate) async fn fetch_raw_move_object<T>(
    rpc: &SuiSDKParamsRPC,
    object_id: ObjectID,
) -> Result<Option<T>, anyhow::Error>
where
    T: DeserializeOwned,
{
    let sui_client = match rpc.sui_client.as_ref() {
        Some(x) => &x.inner,
        None => bail!(DTPError::DTPMissingSuiClient),
    };

    //let object_id_str = object_id.to_string();
    let response = sui_client
        .read_api()
        .get_object_with_options(object_id, SuiObjectDataOptions::default().with_bcs())
        .await;

    if let Err(e) = response {
        return Err(DTPError::DTPFailedFetchObject {
            object_type: std::any::type_name::<T>().to_string(),
            object_id: object_id.to_string(),
            inner: e.to_string(),
        }
        .into());
    }

    let response = response.unwrap().into_object();
    if let Err(e) = response {
        // If the enum 'e' is of type NotExists, or Deleted return Ok(None)
        match e {
            SuiObjectResponseError::NotExists { .. } => return Ok(None),
            SuiObjectResponseError::Deleted { .. } => return Ok(None),
            _ => {
                return Err(DTPError::DTPFailedFetchObject {
                    object_type: std::any::type_name::<T>().to_string(),
                    object_id: object_id.to_string(),
                    inner: e.to_string(),
                }
                .into());
            }
        }
    }
    let resp = response.unwrap();

    // Deserialize the BCS data into T
    let raw_data = resp.to_string(); // Copy to string for debug purpose... optimize this later?
    let sui_raw_data = resp.bcs;
    if let Some(sui_raw_data) = sui_raw_data {
        if let Some(sui_raw_mov_obj) = sui_raw_data.try_into_move() {
            let ret_value: Result<T, anyhow::Error> = sui_raw_mov_obj.deserialize();
            if let Err(e) = ret_value {
                let raw_data = format!("{},inner error[{}]", raw_data, e);
                return Err(DTPError::DTPFailedConvertBCS {
                    object_type: std::any::type_name::<T>().to_string(),
                    object_id: object_id.to_string(),
                    raw_data,
                }
                .into());
            }
            return Ok(Some(ret_value.unwrap()));
        }
    };

    Err(DTPError::DTPFailedConvertBCS {
        object_type: std::any::type_name::<T>().to_string(),
        object_id: object_id.to_string(),
        raw_data,
    }
    .into())
}

pub(crate) async fn fetch_raw_move_object_by_auth<T>(
    rpc: &SuiSDKParamsRPC,
    package_id: &ObjectID,
    module: &str,      // e.g. host
    object_type: &str, // e.g. Host
    auth_address: &SuiAddress,
) -> Result<Option<T>, anyhow::Error>
where
    T: DeserializeOwned,
{
    // Returns Ok(None) when confirmed 'address' does **not** own an instance of T.
    let sui_client = match rpc.sui_client.as_ref() {
        Some(x) => &x.inner,
        None => bail!(DTPError::DTPMissingSuiClient),
    };

    let object_type = format!("{}::{}::{}", package_id, module, object_type);
    let tag = StructTag::from_str(&object_type);
    if let Err(e) = tag {
        return Err(DTPError::DTPFailedFetchObject {
            object_type,
            object_id: "NA".to_string(),
            inner: e.to_string(),
        }
        .into());
    }
    let tag = tag.unwrap();

    let mut objects: Vec<SuiObjectResponse> = Vec::new();
    let mut cursor = None;
    loop {
        let resp = sui_client
            .read_api()
            .get_owned_objects(
                *auth_address,
                Some(SuiObjectResponseQuery::new(
                    Some(SuiObjectDataFilter::StructType(tag.clone())),
                    Some(SuiObjectDataOptions::new().with_bcs()),
                )),
                cursor,
                None,
            )
            .await;

        if let Err(e) = resp {
            return Err(DTPError::DTPFailedFetchObject {
                object_type,
                object_id: "NA".to_string(),
                inner: e.to_string(),
            }
            .into());
        }
        let resp = resp.unwrap();

        objects.extend(resp.data);

        if resp.has_next_page {
            cursor = resp.next_cursor;
        } else {
            break;
        }
    }

    if objects.is_empty() {
        return Ok(None);
    }

    // For now, just pick the first one.
    let response = objects.remove(0).into_object();
    if let Err(e) = response {
        // If the enum 'e' is of type NotExists, or Deleted return Ok(None)
        match e {
            SuiObjectResponseError::NotExists { .. } => return Ok(None),
            SuiObjectResponseError::Deleted { .. } => return Ok(None),
            _ => {
                return Err(DTPError::DTPFailedFetchObject {
                    object_type: std::any::type_name::<T>().to_string(),
                    object_id: "NA".to_string(),
                    inner: e.to_string(),
                }
                .into());
            }
        }
    }
    let resp = response.unwrap();

    // Deserialize the BCS data into T
    let raw_data = resp.to_string(); // Copy to string for debug purpose... optimize this later?
    let sui_raw_data = resp.bcs;
    if let Some(sui_raw_data) = sui_raw_data {
        if let Some(sui_raw_mov_obj) = sui_raw_data.try_into_move() {
            let ret_value: Result<T, anyhow::Error> = sui_raw_mov_obj.deserialize();
            if let Err(e) = ret_value {
                let raw_data = format!("{},inner error[{}]", raw_data, e);
                return Err(DTPError::DTPFailedConvertBCS {
                    object_type: std::any::type_name::<T>().to_string(),
                    object_id: "NA".to_string(),
                    raw_data,
                }
                .into());
            }
            return Ok(Some(ret_value.unwrap()));
        }
    };

    Err(DTPError::DTPFailedConvertBCS {
        object_type: std::any::type_name::<T>().to_string(),
        object_id: "NA".to_string(),
        raw_data,
    }
    .into())
}
