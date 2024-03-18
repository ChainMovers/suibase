// Must match Move object definition(s) on network
use serde::Deserialize;

use sui_sdk::types::base_types::SuiAddress;
use sui_types::id::UID;

#[derive(Deserialize, Debug)]
pub struct UserRegistryMoveRaw {
    pub id: UID,
    pub host_addr: SuiAddress,
}
