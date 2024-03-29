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
//   "TransportControl"
//   "TransportControlInternal"
//   "TransportControlMoveRaw"
//
use crate::types::{DTPError, SuiSDKParamsRPC, SuiSDKParamsTxn};

use super::host_internal::HostInternalST;
use super::{ConnObjectsMoveRaw, ConnReqMoveRaw, LocalhostInternal};

// Stuff needed typically for a Move Call
use serde_json::json;
use std::sync::Arc;
use sui_sdk::json::SuiJsonValue;

use sui_types::base_types::{ObjectID, SuiAddress};

#[derive(Debug, Clone)]
pub struct ConnObjectsInternal {
    // References to all info needed to exchange data
    // through a connection.
    //
    // If an end-point loose these references, they can be
    // re-discovered using one of the related Host object.
    pub tc: ObjectID, // TransportControl
    pub cli_auth: SuiAddress,
    pub srv_auth: SuiAddress,
    pub cli_tx_pipe: ObjectID,
    pub srv_tx_pipe: ObjectID,
    pub cli_tx_ipipes: Vec<ObjectID>,
    pub srv_tx_ipipes: Vec<ObjectID>,
}

pub fn conn_objects_raw_to_internal(
    raw: ConnObjectsMoveRaw,
) -> Result<ConnObjectsInternal, anyhow::Error> {
    let tc = match ObjectID::from_bytes(raw.tc) {
        Ok(x) => x,
        Err(e) => {
            return Err(DTPError::DTPFailedConnObjectsLoading {
                desc: e.to_string(),
            }
            .into())
        }
    };

    let cli_tx_pipe = match ObjectID::from_bytes(raw.cli_tx_pipe) {
        Ok(x) => x,
        Err(e) => {
            return Err(DTPError::DTPFailedConnObjectsLoading {
                desc: e.to_string(),
            }
            .into())
        }
    };
    let srv_tx_pipe = match ObjectID::from_bytes(raw.srv_tx_pipe) {
        Ok(x) => x,
        Err(e) => {
            return Err(DTPError::DTPFailedConnObjectsLoading {
                desc: e.to_string(),
            }
            .into())
        }
    };
    let cli_tx_ipipes: Vec<ObjectID> = raw
        .cli_tx_ipipes
        .iter()
        .map(|x| {
            ObjectID::from_bytes(x).map_err(|e| DTPError::DTPFailedConnObjectsLoading {
                desc: e.to_string(),
            })
        })
        .collect::<Result<Vec<ObjectID>, DTPError>>()?;

    let srv_tx_ipipes: Vec<ObjectID> = raw
        .srv_tx_ipipes
        .iter()
        .map(|x| {
            ObjectID::from_bytes(x).map_err(|e| DTPError::DTPFailedConnObjectsLoading {
                desc: e.to_string(),
            })
        })
        .collect::<Result<Vec<ObjectID>, _>>()?;

    // Convert the raw Move object into the local memory representation.
    Ok(ConnObjectsInternal {
        tc,
        cli_auth: raw.cli_auth,
        srv_auth: raw.srv_auth,
        cli_tx_pipe,
        srv_tx_pipe,
        cli_tx_ipipes,
        srv_tx_ipipes,
    })
}

#[derive(Debug, Clone)]
pub struct TransportControlInternalST {
    service_idx: u8,
    // Correlation ID.
    //
    // Unique for each request within the scope of this process.
    //
    // In other word, it can only be used for matching responses with requests
    // originating from this process.
    cid_cnt: u64,
    // Set when TC confirmed exists on network.
    conn_objects: Option<ConnObjectsInternal>,
}

impl TransportControlInternalST {
    pub fn get_service_idx(&self) -> u8 {
        self.service_idx
    }
    pub fn get_conn_objects(&self) -> Option<ConnObjectsInternal> {
        self.conn_objects.clone()
    }

    pub fn get_tc_address(&self) -> Option<String> {
        if self.conn_objects.is_none() {
            return None;
        }
        let conn_objects = self.conn_objects.as_ref().unwrap();
        Some(conn_objects.tc.to_string())
    }

    pub fn get_next_cid(&mut self) -> u64 {
        self.cid_cnt += 1;
        self.cid_cnt
    }
}

pub type TransportControlInternalMT = Arc<tokio::sync::RwLock<TransportControlInternalST>>;

pub(crate) async fn open_connection_on_network(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    cli_host: &LocalhostInternal,
    srv_host: &HostInternalST,
    service_idx: u8,
) -> Result<TransportControlInternalMT, anyhow::Error> {
    // Creates also the related pipe(s) and inner pipe(s).
    let vargs: Vec<u8> = vec![];
    let call_args = vec![
        SuiJsonValue::new(json!(service_idx))?,
        SuiJsonValue::from_object_id(cli_host.object_id()),
        SuiJsonValue::from_object_id(srv_host.object_id()),
        SuiJsonValue::from_bcs_bytes(None, &vargs).unwrap(),
    ];

    let conn_req_raw = super::common_rpc::do_move_call_ret_event::<ConnReqMoveRaw>(
        rpc,
        txn,
        "api",
        "open_connection",
        "events",
        "ConnReq",
        call_args,
    )
    .await?;

    // Build the internal representation.
    let conn_objs_raw = conn_req_raw.conn;
    let conn_objs = conn_objects_raw_to_internal(conn_objs_raw)?;
    let tci = TransportControlInternalST {
        service_idx,
        cid_cnt: 0,
        conn_objects: Some(conn_objs),
    };

    // All good. Make the TransportControlInternal thread safe.
    Ok(Arc::new(tokio::sync::RwLock::new(tci)))
}

pub(crate) async fn send_request_on_network(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    ipipe: ObjectID,
    data: Vec<u8>,
    cid: u64,
) -> Result<(), anyhow::Error> {
    // Creates also the related pipe(s) and inner pipe(s).
    let vargs: Vec<u8> = vec![];
    let call_args = vec![
        SuiJsonValue::from_object_id(ipipe),
        SuiJsonValue::new(json!(data))?,
        SuiJsonValue::new(json!(cid.to_string()))?, // TODO inefficient conversion, but needed for U64!?!?
        SuiJsonValue::from_bcs_bytes(None, &vargs).unwrap(),
    ];

    super::common_rpc::do_move_call_no_ret(rpc, txn, "api", "send_request", call_args).await
}

pub(crate) async fn send_response_on_network(
    rpc: &SuiSDKParamsRPC,
    txn: &SuiSDKParamsTxn,
    ipipe: ObjectID,
    req_ipipe_idx: u8,
    req_seq_num: u64,
    data: Vec<u8>,
    cid: u64,
) -> Result<(), anyhow::Error> {
    // Creates also the related pipe(s) and inner pipe(s).
    let vargs: Vec<u8> = vec![];
    let call_args = vec![
        SuiJsonValue::from_object_id(ipipe),
        SuiJsonValue::new(json!(req_ipipe_idx.to_string()))?,
        SuiJsonValue::new(json!(req_seq_num.to_string()))?,
        SuiJsonValue::new(json!(data))?,
        SuiJsonValue::new(json!(cid.to_string()))?, // TODO inefficient conversion, but needed for U64!?!?
        SuiJsonValue::from_bcs_bytes(None, &vargs).unwrap(),
    ];

    super::common_rpc::do_move_call_no_ret(rpc, txn, "api", "send_response", call_args).await
}
