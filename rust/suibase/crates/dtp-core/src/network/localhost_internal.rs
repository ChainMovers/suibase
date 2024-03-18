use crate::types::SuiSDKParamsRPC;
use crate::types::SuiSDKParamsTxn;

use super::host_internal::*;

use sui_sdk::json::SuiJsonValue;
use sui_sdk::types::base_types::{ObjectID, SuiAddress};

#[derive(Debug)]
pub struct LocalhostInternal {
    object_id: ObjectID,
    admin_address: SuiAddress,
    firewall_initialized: bool,
    host_internal: HostInternal,
}

pub(crate) async fn get_localhost_internal_by_id(
    rpc: &SuiSDKParamsRPC,
    host_id: ObjectID,
) -> Result<Option<LocalhostInternal>, anyhow::Error> {
    // Do the equivalent of get_host_by_id, but
    // create a handle that will allow for administrator
    // capabilities.
    #[allow(clippy::needless_borrow)]
    let host_internal = super::host_internal::get_host_internal_by_id(rpc, host_id).await?;
    if host_internal.is_none() {
        return Ok(None);
    }
    let host_internal = host_internal.unwrap();

    let localhost_internal = LocalhostInternal {
        object_id: host_id,
        admin_address: rpc.client_address,
        firewall_initialized: false,
        host_internal,
    };

    Ok(Some(localhost_internal))
}

// The host is consumed.
pub(crate) fn create_localhost_from_host(
    rpc: &SuiSDKParamsRPC,
    host_internal: HostInternal,
) -> LocalhostInternal {
    let object_id = host_internal.object_id;
    LocalhostInternal {
        object_id,
        admin_address: rpc.client_address,
        firewall_initialized: false,
        host_internal,
    }
}

pub(crate) async fn create_localhost_on_network(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
) -> Result<LocalhostInternal, anyhow::Error> {
    // Do not allow to create a new one if one already exists
    // for this user.
    let vargs: Vec<u8> = vec![];
    let call_args = vec![SuiJsonValue::from_bcs_bytes(None, &vargs).unwrap()];
    let host_object_id = super::common_rpc::do_move_call_ret_id(
        rpc,
        txn,
        "api",
        "create_host",
        "host",
        "Host",
        call_args,
    )
    .await?;

    // Success.
    Ok(LocalhostInternal {
        object_id: host_object_id,
        admin_address: rpc.client_address,
        firewall_initialized: false,
        host_internal: HostInternal::new(host_object_id),
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
