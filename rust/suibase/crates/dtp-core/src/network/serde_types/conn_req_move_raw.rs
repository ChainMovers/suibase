use serde::Deserialize;

use super::ConnObjectsMoveRaw;

#[derive(Deserialize, Debug)]
pub struct ConnReqMoveRaw {
    pub service_idx: u8,          // Service Type
    pub conn: ConnObjectsMoveRaw, // Info to get the connection started (e.g. Pipes and InnerPipes addresses).
}
