use crate::types::SuiSDKParamsRPC;
use crate::types::SuiSDKParamsTxn;

use super::super::types::error::*;
use super::host_internal::*;

use anyhow::bail;
use shared_crypto::intent::Intent;
use sui_json_rpc_types::SuiTransactionBlockEffectsAPI;
use sui_json_rpc_types::SuiTransactionBlockResponseOptions;
use sui_keys::keystore::AccountKeystore;
use sui_sdk::json::SuiJsonValue;
use sui_sdk::types::base_types::{ObjectID, SuiAddress};
use sui_types::quorum_driver_types::ExecuteTransactionRequestType;
use sui_types::transaction::Transaction;
//use sui_sdk::types::messages::Transaction;
//use sui_types::intent::Intent;
//use sui_types::messages::ExecuteTransactionRequestType;

#[derive(Debug)]
pub struct LocalhostInternal {
    #[allow(dead_code)]
    object_id: ObjectID,
    admin_address: SuiAddress,
    #[allow(dead_code)]
    firewall_initialized: bool,
    #[allow(dead_code)]
    host_internal: HostInternal,
}

pub(crate) async fn get_localhost_by_id(
    rpc: &SuiSDKParamsRPC,
    host_id: ObjectID,
) -> Result<LocalhostInternal, anyhow::Error> {
    // Do the equivalent of get_host_by_id, but
    // create a handle that will allow for administrator
    // capabilities.
    #[allow(clippy::needless_borrow)]
    let host_internal = super::host_internal::get_host_by_id(rpc, host_id).await?;

    let localhost_internal = LocalhostInternal {
        object_id: host_id,
        admin_address: rpc.client_address,
        firewall_initialized: false,
        host_internal,
    };

    Ok(localhost_internal)
}

pub(crate) async fn create_localhost_on_network(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
) -> Result<LocalhostInternal, anyhow::Error> {
    // Do not allow to create a new one if one already exists
    // for this user.

    let sui_client = match rpc.sui_client.as_ref() {
        Some(x) => &x.inner,
        None => bail!(DTPError::DTPMissingSuiClient),
    };
    let keystore = &txn.keystore.inner;

    let vargs: Vec<u8> = vec![];
    let call_args = vec![SuiJsonValue::from_bcs_bytes(None, &vargs).unwrap()];

    let function = "create_host";

    let move_call = sui_client
        .transaction_builder()
        .move_call(
            rpc.client_address,
            txn.package_id,
            "api",
            function,
            vec![],
            call_args,
            None, // The node will pick a gas object belong to the signer if not provided.
            10000000,
        )
        .await
        .map_err(|e| DTPError::DTPFailedMoveCall {
            desc: function.to_string(),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
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
        let mut error_message = "Move Host Create".to_string();
        for error in response.errors {
            error_message.push_str(&error.to_string());
        }
        bail!(DTPError::DTPFailedMoveCall {
            desc: function.to_string(),
            package_id: txn.package_id.to_string(),
            client_address: rpc.client_address.to_string(),
            inner: error_message
        });
    }

    // Get the id from the newly created Sui object.
    let object_id = response
        .effects
        .unwrap()
        .shared_objects()
        .first()
        .unwrap()
        .object_id;

    // Success.
    Ok(LocalhostInternal {
        object_id,
        admin_address: rpc.client_address,
        firewall_initialized: false,
        host_internal: HostInternal::new(object_id),
    })
}

impl LocalhostInternal {
    pub fn get_admin_address(&self) -> &SuiAddress {
        &self.admin_address
    }

    pub(crate) async fn init_firewall(
        &mut self,
        _rpc: &SuiSDKParamsRPC,
        _txn: &SuiSDKParamsTxn,
    ) -> Result<(), anyhow::Error> {
        // Dummy mutable for now... just to test the software design "layering"
        // with a mut.
        self.firewall_initialized = true;
        Ok(())
    }

    pub fn object_id(&self) -> ObjectID {
        self.object_id
    }
}
