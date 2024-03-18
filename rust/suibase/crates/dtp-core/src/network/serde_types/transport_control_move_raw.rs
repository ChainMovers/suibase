// Must match Move object definition(s) on network
use serde::Deserialize;

use sui_sdk::types::base_types::SuiAddress;
use sui_types::id::UID;

use super::super::common_rpc::WeakRef;

#[derive(Deserialize, Debug)]
pub struct TransportControlMoveRaw {
    pub id: UID,

    pub flags: u8, // DTP version+esc flags always after UID.

    pub service_idx: u8, // UDP, Ping, HTTPS etc...

    // Hosts involved in the connection.
    pub cli_host: WeakRef,
    pub srv_host: WeakRef,

    // Some call authorization verified with sender ID address.
    pub cli_addr: SuiAddress,
    pub srv_addr: SuiAddress,

    // Connection Type.
    //
    // TODO Bi-directional, uni-directional or broadcast. Always bi-directional for now.

    // Keep track of the related Pipes.
    //
    // Intended for slow discovery.
    //
    // It is expected that DTP off-chain will cache these IDs.
    pub cli_tx_pipe: WeakRef,
    pub srv_tx_pipe: WeakRef,
}
