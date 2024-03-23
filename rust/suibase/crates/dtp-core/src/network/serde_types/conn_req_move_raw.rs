use serde::Deserialize;
use sui_types::base_types::SuiAddress;

use super::ConnObjectsMoveRaw;

#[derive(Deserialize, Debug)]
pub struct ConnReqMoveRaw {
    pub flags: u8,
    pub src: u8,
    pub src_addr: SuiAddress,     // The server Host address.
    pub service_idx: u8,          // Service Type
    pub conn: ConnObjectsMoveRaw, // Info to get the connection started (e.g. Pipes and InnerPipes addresses).
}
