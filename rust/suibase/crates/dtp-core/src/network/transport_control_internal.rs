use crate::types::{DTPError, SuiSDKParamsRPC, SuiSDKParamsTxn};

use super::host_internal::HostInternal;
use super::LocalhostInternal;

// Stuff needed typically for a Move Call
use std::str::FromStr;
use sui_keys::keystore::AccountKeystore;
use sui_sdk::json::SuiJsonValue;
use sui_sdk::types::base_types::ObjectID;

use shared_crypto::intent::Intent;
use sui_json_rpc_types::{SuiTransactionBlockEffectsAPI, SuiTransactionBlockResponseOptions};
use sui_types::quorum_driver_types::ExecuteTransactionRequestType;
use sui_types::transaction::Transaction;

use anyhow::bail;

// The "internal" object is a private implementation (not intended to be
// directly exposed throught the DTP SDK).
pub struct TransportControlInternal {
    // Set when TC confirmed exists.
    package_id: Option<ObjectID>,
    object_id: Option<ObjectID>,

    // Parameters used when the object was built (only set
    // if was part of a recent operation).
    call_args: Option<Vec<SuiJsonValue>>,
}

pub(crate) async fn create_best_effort_transport_control_on_network(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    localhost: &LocalhostInternal,
    srv_host: &HostInternal,
    _server_protocol: u16,
    _server_port: Option<u16>,
    _return_port: Option<u16>,
) -> Result<TransportControlInternal, anyhow::Error> {
    let sui_client = match rpc.sui_client.as_ref() {
        Some(x) => &x.inner,
        None => bail!(DTPError::DTPMissingSuiClient),
    };
    let keystore = &txn.keystore.inner;

    let server_adm = match srv_host.authority() {
        Some(x) => x,
        None => bail!(DTPError::DTPMissingServerAdminAddress),
    };

    /* Params must match. See transport_control.move
       cli_host: ID,
       srv_host: ID,
       server_adm: address,
       protocol: u16,
       port: u16,
       return_port: u16,
    */

    let call_args = vec![
        SuiJsonValue::from_object_id(localhost.object_id()),
        SuiJsonValue::from_object_id(srv_host.object_id()),
        SuiJsonValue::from_str(&server_adm.to_string()).unwrap(),
        SuiJsonValue::from_str("0").unwrap(),
        SuiJsonValue::from_str("0").unwrap(),
        SuiJsonValue::from_str("0").unwrap(),
    ];

    let function = "create_best_effort";

    let move_call = sui_client
        .transaction_builder()
        .move_call(
            rpc.client_address,
            txn.package_id,
            "transport_control",
            function,
            vec![],
            call_args,
            None, // The node will pick a gas object belong to the signer if not provided.
            1000,
        )
        .await
        .map_err(|e| DTPError::DTPFailedMoveCall {
            desc: function.to_string(),
            client_address: rpc.client_address.to_string(),
            package_id: txn.package_id.to_string(),
            inner: e.to_string(),
        })?;

    // Sign transaction.
    let signature =
        keystore.sign_secure(&rpc.client_address, &move_call, Intent::sui_transaction())?;

    let tx = Transaction::from_data(move_call, vec![signature]);

    let response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            tx,
            SuiTransactionBlockResponseOptions::new().with_effects(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    if !response.errors.is_empty() {
        // Return an anyhow error after adding the response.errors
        // to the error message.
        let mut error_message = "Response Error(s): ".to_string();
        for error in response.errors {
            error_message.push_str(&error.to_string());
        }
        bail!(DTPError::DTPFailedMoveCall {
            desc: function.to_string(),
            client_address: rpc.client_address.to_string(),
            package_id: txn.package_id.to_string(),
            inner: error_message
        });
    }

    let _sui_id = response
        .effects
        .unwrap()
        .shared_objects()
        .first()
        .unwrap()
        .object_id;

    // All good. Build the DTP handles.
    let tci = TransportControlInternal {
        package_id: None,
        object_id: None,
        call_args: None,
    };
    Ok(tci)
}
