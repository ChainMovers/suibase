// Some SUI SDK objects are wrapped.
//

use derive_where::derive_where;
use sui_sdk::types::base_types::{ObjectID, SuiAddress};

// This is for tacking a Debug Trait to Mysten Labs SuiClient
#[derive_where(Debug)]
#[derive_where(skip_inner(Debug))]
pub struct SuiClientWrapped {
    pub inner: sui_sdk::SuiClient,
}

// This is for tacking a Debug Trait to Mysten Labs Keystore
#[derive_where(Debug)]
#[derive_where(skip_inner(Debug))]
pub struct KeystoreWrapped {
    pub inner: sui_keys::keystore::Keystore,
}

// When a function requires SuiSDKParamsRPC you can
// assume that it will make a RPC call.
#[derive(Debug)]
pub struct SuiSDKParamsRPC {
    pub client_address: SuiAddress,
    pub sui_client: Option<SuiClientWrapped>,
}

// When a function take SuiSDKParamsTxn you can
// assume there will be gas cost.
#[derive(Debug)]
pub struct SuiSDKParamsTxn {
    pub package_id: ObjectID,
    pub gas_address: SuiAddress,
    pub keystore: KeystoreWrapped,
}
