use crate::types::{
    DTPError, KeystoreWrapped, PingStats, SuiClientWrapped, SuiSDKParamsRPC, SuiSDKParamsTxn,
};

use log::info;
use std::path::PathBuf;
use sui_keys::keystore::{FileBasedKeystore, Keystore};
use sui_sdk::types::base_types::{ObjectID, SuiAddress};
use sui_sdk::{SuiClient, SuiClientBuilder};

use anyhow::bail;

use super::{HostInternal, LocalhostInternal, TransportControlInternalMT, UserRegistryInternal};

// The default location for localnet is relative to
// this module Cargo.toml location.
//
// TODO Handle default for devnet/testnet ... mainnet.
const DEFAULT_LOCALNET_KEYSTORE_PATHNAME: &str = "../../../dtp-dev/user-localnet/sui.keystore";

// NetworkManager
//
// Perform network objects management associated to a single client address.
// Includes creation, deletion, indexing etc...
//
// A client address should be associated to only one NetworkManager instance (to
// prevent some equivocation scenarios).
//
// A Sui network object can have multiple local handles (say to represent the object
// at different point in time), and any handle can be used to interact with the
// latest version of the object on the network.
//
// For every handles in the API there is a one-to-one relationship with
// an 'Internal' version that encapsulate most of the implementation.
//
// Examples:
//     An API dtp-sdk::Host      --- owns a ----> dtp-core::HostInternal
//     An API dtp-sdk::Locahost  --- owns a ----> dtp-core::LocalhostInternal
//

#[derive(Debug)]
pub struct SuiNode {
    rpc: SuiSDKParamsRPC,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct NetworkManager {
    sui_nodes: Vec<SuiNode>,

    sui_txn: SuiSDKParamsTxn,

    localhost_id: Option<ObjectID>,
    volunteers_id: Vec<ObjectID>,

    // Latest objects loaded from network.
    localhost: Option<LocalhostInternal>,
    registry: Option<UserRegistryInternal>,
}

impl NetworkManager {
    pub async fn new(
        auth_address: SuiAddress,
        keystore_pathname: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        // TODO Extra validation that keystore and client_address are valid.

        // TODO Rewrite the building of the PathBuf for devnet/testnet... mainnet.

        let pathbuf = if let Some(x) = keystore_pathname {
            PathBuf::from(x)
        } else {
            let path = env!("CARGO_MANIFEST_DIR");
            let pathname = format!("{}/{}", path, DEFAULT_LOCALNET_KEYSTORE_PATHNAME);
            PathBuf::from(pathname)
        };

        let keystore = Keystore::File(FileBasedKeystore::new(&pathbuf)?);

        let rpc = SuiSDKParamsRPC {
            client_address: auth_address,
            sui_client: None,
        };

        // TODO Do this here ????
        // Get the package_id from reading the file at:
        //   ~/suibase/workdirs/localnet/published-data/dtp/most-recent/package-id.json
        // Example of content:
        //    ["0x6f3609095927e103a874bc1b82673ff202a42280344fca0054262642c8ed8feb"]
        //
        let txn = SuiSDKParamsTxn {
            package_id: ObjectID::ZERO, // TODO Revisit this when mainnet.
            gas_address: SuiAddress::ZERO,
            keystore: KeystoreWrapped { inner: keystore },
        };

        Ok(NetworkManager {
            sui_nodes: vec![SuiNode { rpc }],
            sui_txn: txn,
            localhost_id: None,
            volunteers_id: Vec::new(),
            localhost: None,
            registry: None,
        })
    }

    // Add RPC details to a Sui node.
    pub async fn add_rpc_url(&mut self, http_url: &str) -> Result<(), anyhow::Error> {
        if self.sui_nodes.is_empty() {
            bail!(DTPError::DTPInternalError {
                msg: "add_rpc_url".to_string()
            })
        }

        if !self.sui_nodes.is_empty() && self.sui_nodes[0].rpc.sui_client.is_some() {
            bail!(DTPError::DTPMultipleRPCNotImplemented)
        }

        let sui_client = SuiClientBuilder::default().build(http_url).await?;
        self.sui_nodes[0].rpc.sui_client = Some(SuiClientWrapped { inner: sui_client });

        // Add event loop handling. For now, simply subscribe to
        // all events touching this client.

        Ok(())
    }

    // Accessors
    pub fn get_auth_address(&self) -> &SuiAddress {
        &self.sui_nodes[0].rpc.client_address
    }
    pub fn get_package_id(&self) -> &ObjectID {
        &self.sui_txn.package_id
    }
    pub fn get_localhost_id(&self) -> &Option<ObjectID> {
        &self.localhost_id
    }

    // TODO Needed?
    pub fn get_sui_client(&self) -> Option<&SuiClient> {
        let sui_client_wrapped = self.sui_nodes[0].rpc.sui_client.as_ref();
        if let Some(sui_client_wrapped) = sui_client_wrapped {
            return Some(&sui_client_wrapped.inner);
        }
        None
    }
    pub fn get_gas_address(&self) -> &SuiAddress {
        &self.sui_txn.gas_address
    }

    // Mutators
    pub fn set_package_id(&mut self, package_id: ObjectID) {
        self.sui_txn.package_id = package_id;
    }

    pub fn set_gas_address(&mut self, gas_address: SuiAddress) {
        self.sui_txn.gas_address = gas_address;
    }

    /*
    pub fn set_localhost_id(&mut self, localhost_id: ObjectID) {
        self.localhost_id = Some(localhost_id);
    }*/

    // Accessors that do a JSON-RPC call.
    pub async fn get_host_by_id(
        &self,
        host_id: ObjectID,
    ) -> Result<Option<HostInternal>, anyhow::Error> {
        super::get_host_internal_by_id(&self.sui_nodes[0].rpc, host_id).await
    }

    pub async fn get_host_by_auth(
        &self,
        address: &SuiAddress,
    ) -> Result<Option<HostInternal>, anyhow::Error> {
        super::get_host_internal_by_auth(&self.sui_nodes[0].rpc, &self.sui_txn.package_id, address)
            .await
    }

    async fn get_localhost_id_from_registry(&mut self) -> Result<Option<ObjectID>, anyhow::Error> {
        // Returns Ok(None) if confirmed there is no registry on network.
        // Uses cached UserRegistryInternal when already loaded.
        let _ = self.load_user_registry().await?;
        if let Some(registry) = &self.registry {
            if let Some(host_id) = registry.localhost_id() {
                return Ok(Some(host_id));
            } else {
                // Some registry but no host_id? Must be a bug.
                bail!(DTPError::DTPInternalError {
                    msg: "get_localhost_id_from_registry".to_string()
                })
            }
        }
        Ok(None)
    }

    async fn load_user_registry(&mut self) -> Result<(), anyhow::Error> {
        // Load the user registry from the network, if not already done.
        // To force an update, look for force_load_user_registry().
        if self.registry.is_none() {
            self.force_load_user_registry().await?;
        }
        Ok(())
    }

    async fn force_load_user_registry(&mut self) -> Result<(), anyhow::Error> {
        // Load the latest user registry from the network, even if already loaded in-memory.
        // If does not exists or on failures, leave the memory version unmodified.
        let new_registry = super::get_user_registry_internal_by_auth(
            &self.sui_nodes[0].rpc,
            &self.sui_txn.package_id,
            self.get_auth_address(),
        )
        .await?;
        if new_registry.is_none() {
            // Registry confirmed to not exists, not an error, just leave the memory version untouched.
            info!("force_load_user_registry: registry does not exists");
            return Ok(());
        }
        let new_registry = new_registry.unwrap();

        if let Some(localhost_id) = new_registry.localhost_id() {
            // Copy the localhost_id from the registry.
            //
            // Note: localhost_id is initialized from multiple place (e.g. on localhost creation).
            //       Therefore, it is possible for the registry not being loaded, yet localhost_id
            //       is already valid. This can also be helpful in future to detect delta.
            self.localhost_id = Some(localhost_id);
            // Finally, initialize the memory version.
            self.registry = Some(new_registry);
            return Ok(());
        }

        Err(DTPError::DTPInternalError {
            msg: "force_load_user_registry".to_string(),
        }
        .into()) // Should never happen.
    }

    pub async fn sync_registry(&mut self) -> Result<(), anyhow::Error> {
        // (1) If there is no self.localhost_id and no registry, then do nothing.
        //
        // (1) If Some(self.localhost_id) because a new localhost has been created
        //     and there is no registry in-memory, then load the registry. Go to (3).
        //     If there is no registry, then create it and return.
        //
        // (3) If Some(UserRegistryInternal.localhost_id), then verify that the self.localhost_id
        //     is matching. If one is none, then update using the other.
        //     If both are set, then check for difference.
        //     Update on the network if UserRegistryInternal was changed.
        //

        if self.localhost_id.is_none() {
            if self.registry.is_none() {
                return Ok(()); // Do nothing.
            }
            // Initialize the localhost_id from the registry.
            self.localhost_id = self.registry.as_ref().unwrap().localhost_id();
        }

        if self.registry.is_none() {
            // Load the registry to check if matching or need to be created.
            let _ = self.load_user_registry().await?;
            if self.registry.is_none() {
                let new_registry = super::create_registry_on_network(
                    &self.sui_nodes[0].rpc,
                    &self.sui_txn,
                    self.localhost_id.unwrap(),
                )
                .await?;
                self.registry = Some(new_registry);
                return Ok(());
            }
        }

        // TODO Logic to update the registry (not needed for now).

        Ok(())
    }

    pub async fn get_localhost_by_auth(&mut self) -> Result<Option<HostInternal>, anyhow::Error> {
        // Note: The returned HostInternal is for the API Host object (which does not own a LocalhostInternal).
        //       Instead, a single instance of LocalhostInternal is cached by the netmgr.

        // Similar to get_host_by_auth, but do a few extra steps
        // Get the id from one of the following source (in order):
        //   - Cached value in NetworkMgr.
        //   - From the registry of the local auth.
        //   - With a fetch of object owned by auth, and pick the first Host found.
        //
        let localhost_id = match self.localhost_id {
            Some(x) => Some(x),
            None => {
                self.load_user_registry().await?;
                self.localhost_id
            }
        };

        let host_internal: Option<HostInternal> = if localhost_id.is_none() {
            let auth_address = self.get_auth_address();
            info!(
                "get_localhost_by_auth from network. Fetch for auth [{}]",
                auth_address
            );
            super::get_host_internal_by_auth(
                &self.sui_nodes[0].rpc,
                &self.sui_txn.package_id,
                auth_address,
            )
            .await?
        } else {
            let localhost_id = localhost_id.unwrap();
            info!(
                "get_localhost_by_auth from network. Fetch for known id [{}]",
                localhost_id
            );
            super::get_host_internal_by_id(&self.sui_nodes[0].rpc, localhost_id).await?
        };

        if host_internal.is_none() {
            info!("get_localhost_by_auth confirm not on network");
            return Ok(None);
        }

        // Initialize the cached localhost.
        let host_internal = host_internal.unwrap();
        let localhost_internal =
            super::create_localhost_from_host(&self.sui_nodes[0].rpc, host_internal);

        let localhost_id = localhost_internal.object_id(); // Copy for later

        self.localhost_id = Some(localhost_internal.object_id());
        self.localhost = Some(localhost_internal);
        info!(
            "get_localhost_by_auth loaded successfully {:?}",
            self.localhost_id
        );

        // Build a HostInternal object for the API.
        // The API can "catch it" as the localhost and give it special handling.
        Ok(Some(HostInternal {
            object_id: localhost_id,
            authority: None,
            raw: None,
        }))
    }

    pub async fn load_local_client_registry(
        &mut self,
    ) -> Result<(HostInternal, LocalhostInternal), anyhow::Error> {
        Err(DTPError::DTPNotImplemented.into())
    }

    // Mutators that do a JSON-RPC call and transaction.
    pub async fn init_firewall(&self) -> Result<(), anyhow::Error> {
        // TODO Verify here client_address == localhost.admin_address
        // Detect user error.
        Ok(())
    }

    pub async fn create_localhost_on_network(&mut self) -> Result<HostInternal, anyhow::Error> {
        // Note: The returned HostInternal is for the API Host object (which does not own a LocalhostInternal).
        //       Instead, a single instance of LocalhostInternal is cached by the netmgr.

        // This function clear all local state and check if a
        // Localhost instance already exists on the network.
        //
        // If there is already one, then it will be reflected in the
        // local state and Err(DTPAlreadyExist) will be returned.
        //
        // If None are found on the network, a new Localhost will
        // tentatively be created.
        self.localhost_id = None;

        // Do a RPC call to get the on-chain state of the registry.
        // If there is no registry, then assume there is no localhost.
        // Both will be created in a single transaction later (to
        // minimize cost and race conditions possibility).
        let _ = self.load_local_client_registry().await;

        // A Localhost is already on the network.
        if let Some(x) = self.localhost_id {
            return Err((DTPError::DTPLocalhostAlreadyExists {
                localhost: x.to_string(),
                client: self.get_auth_address().to_string(),
            })
            .into());
        }

        // Proceed with the creation.
        // TODO Retry once in a controlled manner?
        let localhost =
            super::create_localhost_on_network(&self.sui_nodes[0].rpc, &self.sui_txn).await?;

        let localhost_id = localhost.object_id(); // Copy for later

        self.localhost_id = Some(localhost.object_id());
        self.localhost = Some(localhost);

        // Creation succeeded.
        //
        // Double check to minimize caller having to deal with "race condition". Do a RPC to
        // the fullnode to verify if it reflects the creation. Keep trying for up to 10 seconds.
        //
        // Holding the end-user is not ideal, but will minimize a lot of "user complain" of
        // temporary error that are not really error. Have to give the slowest fullnodes a chance
        // to reflect the network changes.

        // TODO Must be a more robust retry ... no bail on network failure here...
        // self.ensure_localhost_ready().await?;

        // Create a Host for the API user with only the ObjectID set.
        // The API can "catch it" as the localhost and give it special handling.
        Ok(HostInternal {
            object_id: localhost_id,
            authority: None,
            raw: None,
        })
    }

    pub async fn ensure_localhost_ready(&mut self) -> Result<(), anyhow::Error> {
        // Most of the time this function will not detect any problem and quickly return Ok.
        //
        // In rare occasion, may detect a corner case or network disruption may have left
        // things in an unusual state, and some additional RPC might be attempted to recover
        // or return an error for allowing the caller to take action.
        //
        // This function will never cause additional Sui gas expense.

        // Verify that the localhost (Host object) is known, if not, then
        // try to recover by retrieving it now.
        //
        // This might have happen if this is the very first time the user
        // is using DTP and have just created the Localhost object on the Sui
        // network (so its object id is known upon creation) but have never
        // use it yet (so the object fields were never retrieve!).
        if self.localhost.is_none() {
            // This should initialize self.localhost and self.localhost_id
            let _ = self.get_localhost_by_auth().await?;
        }

        // Last check to confirm.
        if self.localhost.is_none() || self.get_localhost_id().is_none() {
            bail!(DTPError::DTPLocalhostDoesNotExists)
        }

        Ok(())
    }

    pub async fn ping_on_network(
        &mut self,
        target_host: &HostInternal,
    ) -> Result<PingStats, anyhow::Error> {
        self.ensure_localhost_ready().await?;

        // unwrap() will not fail because ensure_localhost_ready()
        let localhost = self.localhost.as_ref().unwrap();

        // Create connection.
        let mut _tci = super::open_connection_on_network(
            &self.sui_nodes[0].rpc,
            &self.sui_txn,
            localhost,
            target_host,
            7,
        )
        .await?;

        // Make sure this DTP client
        let stats = PingStats {
            ping_count_attempted: 1,
            ..Default::default()
        };

        Ok(stats)
    }

    pub async fn create_connection(
        &mut self,
        target_host: &HostInternal,
        service_idx: u8,
    ) -> Result<TransportControlInternalMT, anyhow::Error> {
        // Creates a new connection even if one already exists on the network.
        self.ensure_localhost_ready().await?;

        // unwrap() will not fail because ensure_localhost_ready()
        let localhost = self.localhost.as_ref().unwrap();

        let tci = super::open_connection_on_network(
            &self.sui_nodes[0].rpc,
            &self.sui_txn,
            localhost,
            target_host,
            service_idx,
        )
        .await?;

        Ok(tci)
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantiate_network_manager() -> Result<(), anyhow::Error> {
        // TODO
        Ok(())
    }

    #[test]
    fn instantiate_hostinternal() -> Result<(), anyhow::Error> {
        // TODO
        Ok(())
    }

    #[test]
    fn instantiate_localhostinternal() -> Result<(), anyhow::Error> {
        // TODO
        Ok(())
    }
}*/
