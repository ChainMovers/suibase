//! suibase
//!
//! API to suibase ( https://suibase.io ) mostly intended for development of Sui tool/test automation and production backends.
//!
//! This API is:
//!   * multi-thread safe.
//!   * UniFFI bindings compatible (Sync+Send)
//!
//! You need to install Suibase first, which is both easy and non-conflicting with your existing Sui installation.
//!
//! see https://suibase.io for instructions.
//!
//! What is Suibase?
//!
//! Suibase makes it easy for Sui devs to simultaneoulsly interact with multiple Sui
//! networks (localnet/devnet/testnet/mainnet) without having to "switch env".
//!
//! Your dev setup gains stability by having a client binary match every network version.
//!

mod error;
pub use crate::error::Error;

mod suibase_helper_impl;
mod suibase_root;
mod suibase_workdir;

use crate::suibase_helper_impl::SuibaseHelperImpl;

use std::sync::{Arc, Mutex};
use sui_types::base_types::{ObjectID, SuiAddress};

#[cfg(feature = "build-with-uniffi")]
uniffi::include_scaffolding!("suibase");
/// A lightweight API to suibase. Multiple instance can be created within the same app.
///
/// You interact with Suibase in 3 steps:
///
///  (1) Check if suibase is_installed()
///  (2) Call select_workdir() to pick among "localnet", "devnet", "testnet", "mainnet" or the one currently set "active" by the user.
///  (3) You can now call any other API functions (in any order). Most calls will relate to the selected workdir.
///
/// You can call again select_workdir() to switch to another workdir.
pub struct Helper(Arc<Mutex<SuibaseHelperImpl>>);

/// This is the documentation for the impl Default
///
impl Default for Helper {
    fn default() -> Self {
        Self::new()
    }
}

/// Comment for the impl Helper Do we care?
impl Helper {
    /// Constructs a new `Helper`.
    ///
    /// # Example
    ///
    /// ```
    /// use suibase::Helper;
    ///
    /// let sbh = Helper::new();
    /// ``
    pub fn new() -> Self {
        Helper(Arc::new(Mutex::new(SuibaseHelperImpl::new())))
    }

    /// Check first if suibase is installed, otherwise
    /// most of the other calls will fail in some ways.
    pub fn is_installed(&self) -> Result<bool, Error> {
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
    pub fn select_workdir(&self, workdir_name: &str) -> Result<(), Error> {
        self.0.lock().unwrap().select_workdir(workdir_name)
    }

    // Get the name of the selected workdir.
    pub fn workdir(&self) -> Result<String, Error> {
        self.0.lock().unwrap().workdir()
    }

    // Get the pathname of the file keystore (when available).
    //
    // Context: Selected Workdir by this API.
    pub fn keystore_pathname(&self) -> Result<String, Error> {
        self.0.lock().unwrap().keystore_pathname()
    }

    // Get the ObjectID of the last successfully published "package_name".
    //
    // package_name is the "name" field specified in the "Move.toml".
    //
    // Related path: ~/suibase/workdirs/<workdir_name>/published-data/<package_name>/
    pub fn package_object_id(&self, package_name: &str) -> Result<ObjectID, Error> {
        self.0.lock().unwrap().package_object_id(package_name)
    }

    // Alternative for string-based API.
    pub fn package_id(&self, package_name: &str) -> Result<String, Error> {
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
    pub fn published_new_object_ids(&self, object_type: &str) -> Result<Vec<ObjectID>, Error> {
        self.0.lock().unwrap().published_new_object_ids(object_type)
    }

    // Alternative for string-based API.
    pub fn published_new_objects(&self, object_type: &str) -> Result<Vec<String>, Error> {
        let res = self.published_new_object_ids(object_type)?;
        Ok(res.iter().map(|c| c.to_string()).collect())
    }

    // Get an address by name.
    //
    // Suibase localnet/devnet/testnet/mainnet workdir are created with a set of pre-defined client addresses.
    //
    // These addresses are useful for testing. In particular, with localnet they are prefunded.
    //
    // Names can be:  active | sb-[1-5]-[ed25519|scp256k1|scp256r1]
    //
    // Examples: "active", "sb-1-ed25519", "sb-3-scp256r1", "sb-5-scp256k1" ...
    //
    // "active" is same as doing "sui client active-address" for the selected workdir.
    //
    pub fn client_sui_address(&self, address_name: &str) -> Result<SuiAddress, Error> {
        self.0.lock().unwrap().client_sui_address(address_name)
    }

    // Alternative for string-based API.
    pub fn client_address(&self, address_name: &str) -> Result<String, Error> {
        let addr = self.client_sui_address(address_name)?;
        Ok(addr.to_string())
    }

    // Get a RPC URL for the selected workdir.
    pub fn rpc_url(&self) -> Result<String, Error> {
        self.0.lock().unwrap().rpc_url()
    }

    // Get a Websocket URL for the selected workdir.
    pub fn ws_url(&self) -> Result<String, Error> {
        self.0.lock().unwrap().ws_url()
    }
}
