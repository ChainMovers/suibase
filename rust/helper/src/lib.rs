//  helper library
//
// Help automate localnet/devnet/testnet operations in a suibase environment.
//
// Your app can select to interact with any of the workdir installed with suibase.
//
// This API is multi-thread safe.
//
// UniFFI bindings compatible (Sync+Send)

mod error;
mod sui_base_helper_impl;
mod sui_base_root;
mod sui_base_workdir;

use crate::error::SuiBaseError;
use crate::sui_base_helper_impl::SuiBaseHelperImpl;

use std::sync::{Arc, Mutex};
use sui_types::base_types::{ObjectID, SuiAddress};

#[cfg(feature = "build-with-uniffi")]
uniffi::include_scaffolding!("suibase");
pub struct SuiBaseHelper(Arc<Mutex<SuiBaseHelperImpl>>);

impl Default for SuiBaseHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl SuiBaseHelper {
    // Steps to get started with the API:
    //
    //  (1) Check if is_installed()
    //
    //  (2) Call select_workdir()
    //
    //  (3) You can now call any other API functions (in any order).
    //      Most calls will relate to the selected workdir.
    pub fn new() -> Self {
        SuiBaseHelper(Arc::new(Mutex::new(SuiBaseHelperImpl::new())))
    }

    // Check first if suibase is installed, otherwise
    // most of the other calls will fail in some ways.
    pub fn is_installed(&self) -> Result<bool, SuiBaseError> {
        self.0.lock().unwrap().is_installed()
    }

    // Select an existing workdir by name.
    //
    // Possible values are:
    //   "active", "cargobin", "localnet", "devnet", "testnet", "mainnet" and
    //    other custom names might be supported in future.
    //
    // Note: "active" is special. It will resolve the active workdir at the moment of the
    //       call. Example: if "localnet" is the active, then this call is equivalent to
    //       to be done for "localnet". The selection does not change even if the user
    //       externally change the active after this call.
    //
    pub fn select_workdir(&self, workdir_name: &str) -> Result<(), SuiBaseError> {
        self.0.lock().unwrap().select_workdir(workdir_name)
    }

    // Get the name of the selected workdir.
    pub fn workdir(&self) -> Result<String, SuiBaseError> {
        self.0.lock().unwrap().workdir()
    }

    // Get the pathname of the file keystore (when available).
    //
    // Context: Selected Workdir by this API.
    pub fn keystore_pathname(&self) -> Result<String, SuiBaseError> {
        self.0.lock().unwrap().keystore_pathname()
    }

    // Get the ObjectID of the last successfully published "package_name".
    //
    // package_name is the "name" field specified in the "Move.toml".
    //
    // Related path: ~/suibase/workdirs/<workdir_name>/published-data/<package_name>/
    pub fn package_object_id(&self, package_name: &str) -> Result<ObjectID, SuiBaseError> {
        self.0.lock().unwrap().package_object_id(package_name)
    }

    // Alternative for string-based API.
    pub fn package_id(&self, package_name: &str) -> Result<String, SuiBaseError> {
        let id = self.package_object_id(package_name)?;
        Ok(id.to_string())
    }

    // Get the ObjectID of the objects that were created when the package was published.
    //
    // object_type format is the Sui Move "package::module::type".
    //
    // Example:
    //
    //    module acme::Tools {
    //       struct Anvil has key, drop { ... }
    //       ...
    //       fun init(ctx: &mut TxContext) {
    //          Anvil::new(ctx);
    //          ...
    //       }
    //    }
    //
    // The object_type is "acme::Tools::Anvil"
    //
    // Related path: ~/suibase/workdirs/<workdir_name>/published-data/<package_name>/
    pub fn published_new_object_ids(
        &self,
        object_type: &str,
    ) -> Result<Vec<ObjectID>, SuiBaseError> {
        self.0.lock().unwrap().published_new_object_ids(object_type)
    }

    // Alternative for string-based API.
    pub fn published_new_objects(&self, object_type: &str) -> Result<Vec<String>, SuiBaseError> {
        let res = self.published_new_object_ids(object_type)?;
        Ok(res.iter().map(|c| c.to_string()).collect())
    }

    // Get an address by name.
    //
    // Suibase localnet/devnet/testnet workdir are created with a set of pre-defined client addresses.
    //
    // These addresses are useful for testing. In particular, with localnet they are prefunded.
    //
    // Names can be:  active | sb-[1-5]-[ed25519|scp256k1|scp256r1]
    //
    // Examples: "active", "sb-1-ed25519", "sb-3-scp256r1", "sb-5-scp256k1" ...
    //
    // "active" is same as doing "sui client active-address" for the selected workdir.
    //
    pub fn client_sui_address(&self, address_name: &str) -> Result<SuiAddress, SuiBaseError> {
        self.0.lock().unwrap().client_sui_address(address_name)
    }

    // Alternative for string-based API.
    pub fn client_address(&self, address_name: &str) -> Result<String, SuiBaseError> {
        let addr = self.client_sui_address(address_name)?;
        Ok(addr.to_string())
    }

    // Get a RPC URL for the selected workdir.
    pub fn rpc_url(&self) -> Result<String, SuiBaseError> {
        self.0.lock().unwrap().rpc_url()
    }

    // Get a Websocket URL for the selected workdir.
    pub fn ws_url(&self) -> Result<String, SuiBaseError> {
        self.0.lock().unwrap().ws_url()
    }
}
