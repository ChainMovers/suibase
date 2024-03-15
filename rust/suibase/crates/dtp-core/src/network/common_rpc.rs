// Utility function to do common RPC calls.

use std::str::FromStr;

use anyhow::bail;
use log::info;
use move_core_types::language_storage::StructTag;
use shared_crypto::intent::Intent;
use sui_json_rpc_types::{
    SuiData, SuiObjectDataFilter, SuiObjectDataOptions, SuiObjectResponse, SuiObjectResponseQuery,
    SuiTransactionBlockResponseOptions, SuiTypeTag,
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

// Function that returns a new object created.
pub(crate) async fn do_move_object_create(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    call_module: &str,            // e.g. api
    function: &str,               // e.g. create
    new_object_module: &str,      // e.g. host
    new_object_type: &str,        // e.g. Host
    call_args: Vec<SuiJsonValue>, // Can be empty vec![]
) -> Result<ObjectID, anyhow::Error> {
    // TODO Add this to call_args at caller point.
    //let vargs: Vec<u8> = vec![];
    //vec![SuiJsonValue::from_bcs_bytes(None, &vargs).unwrap()];

    // Do not allow to create a new one if one already exists
    // for this user.
    let sui_client = match rpc.sui_client.as_ref() {
        Some(x) => &x.inner,
        None => bail!(DTPError::DTPMissingSuiClient),
    };
    let keystore = &txn.keystore.inner;

    let call_desc = format!(
        "{}::{}::{}({:?}) with signer {} to create object {}:{}",
        txn.package_id,
        call_module,
        function,
        call_args,
        rpc.client_address,
        new_object_module,
        new_object_type
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
            None, // The node will pick a gas object belong to the signer if not provided.
            10000000,
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
            SuiTransactionBlockResponseOptions::new()
                .with_object_changes()
                .with_effects(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await;
    if response.is_err() {
        return Err(DTPError::DTPFailedMoveCall {
            desc: format!("response failed for {}", call_desc),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
            inner: format!("Inner error [{}]", response.unwrap_err().to_string()),
        }
        .into());
    }
    let response = response.unwrap();

    if !response.errors.is_empty() {
        let mut error_message = "Inner error [".to_string();
        for error in response.errors {
            error_message.push_str(&error.to_string());
        }
        error_message.push_str("]");

        bail!(DTPError::DTPFailedMoveCall {
            desc: format!("response errors for {}", call_desc),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
            inner: error_message
        });
    }

    // Get the id from the newly created Sui object.
    let object_changes = response.object_changes.unwrap();

    // Iterate the object changes, look for the "host::Host" object.
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
            desc: format!("response object not found for {}", call_desc),
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

    // Check if the response is OK, except that the object does not exists.
    /*
    if !resp.errors.is_empty() {
        let mut error_message = "Inner error [".to_string();
        for error in response.errors {
            error_message.push_str(&error.to_string());
        }
        error_message.push_str("]");
        bail!(DTPError::DTPFailedFetchObject {
            object_type: std::any::type_name::<T>().to_string(),
            object_id: object_id.to_string(),
            inner: error_message
        });
    }*/

    // Deserialize the BCS data into T
    let raw_data = resp.to_string(); // Copy to string for debug purpose... optimize this later?
    let sui_raw_data = resp.bcs;
    if let Some(sui_raw_data) = sui_raw_data {
        if let Some(sui_raw_mov_obj) = sui_raw_data.try_into_move() {
            let ret_value: Result<T, anyhow::Error> = sui_raw_mov_obj.deserialize();
            if let Err(e) = ret_value {
                let raw_data = format!("{},inner error[{}]", raw_data, e.to_string());
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
    info!("fetch_raw_move_object_by_auth start");
    // Returns Ok(None) when confirmed 'address' does **not** own an instance of T.
    let sui_client = match rpc.sui_client.as_ref() {
        Some(x) => &x.inner,
        None => bail!(DTPError::DTPMissingSuiClient),
    };

    let object_type = format!("{}::{}::{}", package_id.to_string(), module, object_type);
    info!(
        "fetch_raw_move_object_by_auth: object_type: {}",
        object_type
    );
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

    info!("fetch_raw_move_object_by_auth A");
    let mut objects: Vec<SuiObjectResponse> = Vec::new();
    let mut cursor = None;
    loop {
        let resp = sui_client
            .read_api()
            .get_owned_objects(
                auth_address.clone(),
                Some(SuiObjectResponseQuery::new(
                    Some(SuiObjectDataFilter::StructType(tag.clone())),
                    Some(SuiObjectDataOptions::new().with_bcs()),
                )),
                cursor.clone(),
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

    info!("fetch_raw_move_object_by_auth B");
    if objects.is_empty() {
        info!("fetch_raw_move_object_by_auth C");
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
    info!("fetch_raw_move_object_by_auth D");

    // Deserialize the BCS data into T
    let raw_data = resp.to_string(); // Copy to string for debug purpose... optimize this later?
    let sui_raw_data = resp.bcs;
    if let Some(sui_raw_data) = sui_raw_data {
        info!("fetch_raw_move_object_by_auth E");
        if let Some(sui_raw_mov_obj) = sui_raw_data.try_into_move() {
            info!("fetch_raw_move_object_by_auth F");
            let ret_value: Result<T, anyhow::Error> = sui_raw_mov_obj.deserialize();
            if let Err(e) = ret_value {
                info!("fetch_raw_move_object_by_auth G");
                let raw_data = format!("{},inner error[{}]", raw_data, e.to_string());
                return Err(DTPError::DTPFailedConvertBCS {
                    object_type: std::any::type_name::<T>().to_string(),
                    object_id: "NA".to_string(),
                    raw_data,
                }
                .into());
            }
            info!("fetch_raw_move_object_by_auth end");
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
