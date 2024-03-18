// Must match Move object definition(s) on network
use serde::Deserialize;

use sui_types::id::UID;

use crate::network::{common_rpc::WeakRef, PipeSyncDataMoveRaw};

#[derive(Deserialize, Debug)]
pub struct PipeMoveRaw {
    pub id: UID,
    pub flgs: u8, // DTP version+esc flags always after UID.

    pub sync_data: PipeSyncDataMoveRaw, // Merged of all InnerPipe sync_data.

    pub tc: WeakRef,          // TransportControl
    pub ipipes: Vec<WeakRef>, // InnerPipe(s)
}
