// Must match Move object definition(s) on network
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PipeSyncDataMoveRaw {
    pub byte_payload_sent: u64,
    pub byte_header_sent: u64,
    pub send_call_completed: u64,
}
