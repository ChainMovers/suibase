// sui-base-helper library
//
// Help automate localnet/devnet/testnet operations in a sui-base environment.
//
// Your app can select to interact with any of the workdir installed with sui-base.
//
mod sui_base_root;
mod sui_base_workdir;

use sui_sdk::types::base_types::{ObjectID, SuiAddress};

use anyhow::{bail, Result};

use crate::sui_base_root::SuiBaseRoot;
use crate::sui_base_workdir::SuiBaseWorkdir;

pub struct SuiBaseHelper {
    root: SuiBaseRoot,               // for most features related to ~/sui-base
    workdir: Option<SuiBaseWorkdir>, // for *one* selected workdir under ~/sui-base/workdirs
}

impl Default for SuiBaseHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl SuiBaseHelper {
    pub fn new() -> SuiBaseHelper {
        SuiBaseHelper {
            root: SuiBaseRoot::new(),
            workdir: None,
        }
    }

    // Check first if sui-base is installed, otherwise
    // most of the other call will fail in some ways.
    pub fn is_sui_base_installed(self: &mut SuiBaseHelper) -> Result<bool, anyhow::Error> {
        Ok(self.root.is_sui_base_installed())
    }

    // Select an existing workdir by name.
    //
    // Possible values are:
    //   "cargobin", "localnet", "devnet", "testnet", "mainnet" and
    //    other custom names might be supported in future.
    //
    pub fn select_workdir(
        self: &mut SuiBaseHelper,
        workdir_name: &str,
    ) -> Result<(), anyhow::Error> {
        let mut new_wd = SuiBaseWorkdir::new();
        new_wd.init_from_existing(&mut self.root, workdir_name)?;
        self.workdir = Some(new_wd);
        Ok(())
    }

    // Use the same workdir as the one currently active for asui.
    //
    // The selection is done for this app at the time of this call.
    //
    // Further "external" changes to asui by the user, after this
    // call, have no effect on this app selection.
    //
    // The name of the selected workdir is returned.
    pub fn select_active_workdir(self: &mut SuiBaseHelper) -> Result<String, anyhow::Error> {
        //let new_wd = SuiBaseWorkdir::new();
        //self.workdir = Some(new_wd.init_from_asui(self.root)?);
        bail!("sui-base: function not implemented. software error.");
    }

    // Get the name of the selected workdir.
    pub fn get_workdir_name(&self) -> Result<String, anyhow::Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.get_name()?),
            None => bail!("sui-base: workdir not selected."),
        }
    }

    // Get the pathname of a file keystore (when available).
    pub fn get_keystore_pathname(&mut self) -> Result<String, anyhow::Error> {
        // TODO Implement this better with sui-base.yaml and/or ENV variables.
        //      See https://github.com/sui-base/sui-base/issues/6
        match &self.workdir {
            Some(wd) => Ok(wd.get_keystore_pathname(&mut self.root)?),
            None => bail!("sui-base: workdir not selected."),
        }
    }

    // Get the ObjectID of the last successfully published "package_name".
    //
    // package_name is the "name" field specified in the "Move.toml".
    //
    pub fn get_package_id(
        self: &mut SuiBaseHelper,
        package_name: &str,
    ) -> Result<ObjectID, anyhow::Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.get_package_id(&mut self.root, package_name)?),
            None => bail!("sui-base: workdir not selected."),
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
    pub fn get_published_new_objects(
        self: &mut SuiBaseHelper,
        object_type: &str,
    ) -> Result<Vec<ObjectID>, anyhow::Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.get_published_new_objects(&mut self.root, object_type)?),
            None => bail!("sui-base: workdir not selected."),
        }
    }

    // Get an address by name.
    //
    // Sui-base localnet/devnet/testnet workdir are created with a set of pre-defined client addresses.
    //
    // These addresses are useful for testing. In particular, with localnet they are prefunded.
    //
    // Their names are:
    //   sb-[1-5]-[ed25519|scp256k1|scp256r1]
    //
    // Example of valid names: "sb-1-ed25519", "sb-3-scp256r1", "sb-5-scp256k1" ...
    //
    pub fn get_client_address(
        self: &mut SuiBaseHelper,
        address_name: &str,
    ) -> Result<SuiAddress, anyhow::Error> {
        match &self.workdir {
            Some(wd) => Ok(wd.get_client_address(&mut self.root, address_name)?),
            None => bail!("sui-base: workdir not selected."),
        }
    }
}
