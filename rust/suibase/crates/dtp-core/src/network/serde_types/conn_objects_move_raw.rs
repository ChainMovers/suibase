use serde::Deserialize;

use sui_sdk::types::base_types::SuiAddress;

#[derive(Deserialize, Debug)]
pub struct ConnObjectsMoveRaw {
    // References to all objects needed to exchange data
    // through a connection.
    //
    // If an end-point loose these references, they can be
    // re-discovered using one of the related Host object.
    pub tc: SuiAddress, // TransportControl
    pub cli_auth: SuiAddress,
    pub srv_auth: SuiAddress,
    pub cli_tx_pipe: SuiAddress,
    pub srv_tx_pipe: SuiAddress,
    pub cli_tx_ipipes: Vec<SuiAddress>, // Note: req_ipipe_idx works on this vector.
    pub srv_tx_ipipes: Vec<SuiAddress>,
}
