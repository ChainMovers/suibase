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

use sui_types::base_types::ObjectID;

#[derive(Debug)]
pub struct ConnObjectsInternal {
    // References to all objects needed to exchange data
    // through a connection.
    //
    // If an end-point loose these references, they can be
    // re-discovered using one of the related Host object.
    pub tc: ObjectID, // TransportControl
    pub cli_tx_pipe: ObjectID,
    pub srv_tx_pipe: ObjectID,
    pub cli_tx_ipipes: Vec<ObjectID>,
    pub srv_tx_ipipes: Vec<ObjectID>,
}

impl ConnObjectsInternal {
    pub fn raw_to_internal(raw: ConnObjectsMoveRaw) -> Result<ConnObjectsInternal, anyhow::Error> {
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
            cli_tx_pipe,
            srv_tx_pipe,
            cli_tx_ipipes,
            srv_tx_ipipes,
        })
    }
}
#[derive(Debug)]
pub struct TransportControlInternalST {
    // Set when TC confirmed exists on network.
    conn_objs: Option<ConnObjectsInternal>,
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
    let conn_objs = ConnObjectsInternal::raw_to_internal(conn_objs_raw)?;
    let tci = TransportControlInternalST {
        conn_objs: Some(conn_objs),
    };

    // All good. Make the TransportControlInternal thread safe.
    Ok(Arc::new(tokio::sync::RwLock::new(tci)))
}
