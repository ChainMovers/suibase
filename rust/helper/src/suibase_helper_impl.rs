// suibase helper library
//
// This is the implementation. See lib.rs for the public API and documentation.

use sui_types::base_types::{ObjectID, SuiAddress};

use crate::error::Error;
use crate::suibase_root::SuibaseRoot;
use crate::suibase_workdir::SuibaseWorkdir;

pub struct SuibaseHelperImpl {
    root: SuibaseRoot,               // for most features related to ~/suibase
    workdir: Option<SuibaseWorkdir>, // for *one* selected workdir under ~/suibase/workdirs
}

impl Default for SuibaseHelperImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl SuibaseHelperImpl {
    // Steps to get started with the API:
    //
    //  (1) Check if is_installed()
    //
    //  (2) Call select_workdir()
    //
    //  (3) You can now call any other API functions (in any order).
    //      Most calls will relate to the selected workdir.

    pub fn new() -> SuibaseHelperImpl {
        SuibaseHelperImpl {
            root: SuibaseRoot::new(),
            workdir: None,
        }
    }

    // Check first if suibase is installed, otherwise
    // most of the other calls will fail in some ways.
    pub fn is_installed(self: &mut SuibaseHelperImpl) -> Result<bool, Error> {
        Ok(self.root.is_installed())
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
    pub fn select_workdir(self: &mut SuibaseHelperImpl, workdir_name: &str) -> Result<(), Error> {
        let mut new_wd = SuibaseWorkdir::new();
        new_wd.init_from_existing(&mut self.root, workdir_name)?;
        self.workdir = Some(new_wd);
        Ok(())
    }

    // Get the name of the selected workdir.
    pub fn workdir(&self) -> Result<String, Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.get_name()?),
            None => Err(Error::WorkdirNotSelected),
        }
    }

    // Get the pathname of the file keystore (when available).
    //
    // Context: Selected Workdir by this API.
    pub fn keystore_pathname(&mut self) -> Result<String, Error> {
        // TODO Implement this better with suibase.yaml and/or ENV variables.
        //      See https://github.com/chainmovers/suibase/issues/6
        match &self.workdir {
            Some(wd) => Ok(wd.keystore_pathname(&mut self.root)?),
            None => Err(Error::WorkdirNotSelected),
        }
    }

    // Get the ObjectID of the last successfully published "package_name".
    //
    // package_name is the "name" field specified in the "Move.toml".
    //
    // Related path: ~/suibase/workdirs/<workdir_name>/published-data/<package_name>/
    pub fn package_object_id(
        self: &mut SuibaseHelperImpl,
        package_name: &str,
    ) -> Result<ObjectID, Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.package_object_id(&mut self.root, package_name)?),
            None => Err(Error::WorkdirNotSelected),
        }
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
        self: &mut SuibaseHelperImpl,
        object_type: &str,
    ) -> Result<Vec<ObjectID>, Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.published_new_object_ids(&mut self.root, object_type)?),
            None => Err(Error::WorkdirNotSelected),
        }
    }

    // Get an address by name.
    //
    // Suibase localnet/devnet/testnet/mainnet workdir are created with a set of pre-defined client addresses.
    //
    // These addresses are useful for testing. In particular, with localnet they are prefunded.
    //
    // Their names are:
    //   sb-[1-5]-[ed25519|scp256k1|scp256r1]
    //
    // Example of valid names: "sb-1-ed25519", "sb-3-scp256r1", "sb-5-scp256k1" ...
    //
    pub fn client_sui_address(
        self: &mut SuibaseHelperImpl,
        address_name: &str,
    ) -> Result<SuiAddress, Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.client_sui_address(&mut self.root, address_name)?),
            None => Err(Error::WorkdirNotSelected),
        }
    }

    // Get a RPC URL for the selected workdir.
    pub fn rpc_url(&mut self) -> Result<String, Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.rpc_url(&mut self.root)?),
            None => Err(Error::WorkdirNotSelected),
        }
    }

    // Get a Websocket URL for the selected workdir.
    pub fn ws_url(&mut self) -> Result<String, Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.ws_url(&mut self.root)?),
            None => Err(Error::WorkdirNotSelected),
        }
    }
}
