// DTP SDK API
//
// Example of use (simplified):
//
//      let dtp = DTP::new(client_address, keystore).await?;
//
//      dtp.add_url( ... your favorite fullnode ip... );
//
//      dtp.create_host(); // Create your own Host object!
//
//      let another_host = dtp.get_host_by_id(...); // Get someone else Host!
//
//      dtp.ping( another_host ); // Ping it!
//
// For most app, only one instance of DTP object will be needed but
// multiple instance should work.
//
// There is a one-to-one relationship between a Sui client address
// and a DTP instance.
//
// Sui SDK and DTP SDK can co-exist and be used independently.

use anyhow::bail;
use dtp_core::{
    network::{HostInternal, NetworkManager},
    types::PingStats,
};
use sui_sdk::types::base_types::{ObjectID, SuiAddress};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Host {
    id: ObjectID,
    host_internal: HostInternal, // Implementation hidden in dtp-core.
}
impl Host {
    pub fn id(&self) -> &ObjectID {
        &self.id
    }
}

pub struct DTP {
    netmgr: NetworkManager, // Implementation hidden in dtp-core.
}

impl DTP {
    pub async fn new(
        client_address: SuiAddress,
        keystore_pathname: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        Ok(DTP {
            #[allow(clippy::needless_borrow)]
            netmgr: NetworkManager::new(client_address, keystore_pathname).await?,
        })
    }

    // Light Mutators
    //   JSON-RPC: No
    //   Gas Cost: No
    pub fn set_package_id(&mut self, package_id: ObjectID) {
        self.netmgr.set_package_id(package_id);
    }

    // Light Accessors
    //   JSON-RPC: No
    //   Gas Cost: No
    pub fn package_id(&self) -> &ObjectID {
        self.netmgr.get_package_id()
    }

    pub fn client_address(&self) -> &SuiAddress {
        self.netmgr.get_client_address()
    }

    pub fn localhost_id(&self) -> &Option<ObjectID> {
        self.netmgr.get_localhost_id()
    }

    pub async fn add_rpc_url(&mut self, http_url: &str) -> Result<(), anyhow::Error> {
        self.netmgr.add_rpc_url(http_url).await
    }

    // get_host
    //   JSON-RPC: Yes
    //   Gas Cost: No
    //
    // Get the Host associated with the current client address (this DTP instance)
    //
    // On success it means the caller have administrative capability on that Host object.
    // (can setup firewall, enable services etc...)
    pub async fn get_host(&self) -> Result<Host, anyhow::Error> {
        if (*self.netmgr.get_localhost_id()).is_none() {
            bail!("Create localhost object first")
        }
        let host_internal = self
            .netmgr
            .get_host(self.netmgr.get_localhost_id().unwrap())
            .await?;
        Ok(Host {
            id: host_internal.object_id(),
            host_internal,
        })
    }

    // get_host_by_id
    //   JSON-RPC: Yes
    //   Gas Cost: No
    //
    // Get an handle of any DTP Host expected to be already on the Sui network.
    //
    // The handle is used for doing various operations such as pinging the host, make
    // RPC calls and/or create connections to it.
    pub async fn get_host_by_id(&self, host_id: ObjectID) -> Result<Host, anyhow::Error> {
        let host_internal = self.netmgr.get_host(host_id).await?;
        Ok(Host {
            id: host_internal.object_id(),
            host_internal,
        })
    }

    // create_host_on_network
    //
    //   JSON-RPC: Yes
    //   Gas Cost: Yes
    //
    // Create a new DTP Host on the Sui network.
    //
    // The Host shared objects created on the network are retrievable
    // as a read-only DTP::Host by everyone with get_host_by_id()
    //
    // To edit/modify the Host shared object, the DTP application
    // must have the administrator capability for it. Any DTP
    // application with the same client address and keystore as
    // the creator of the DTP Host object has such capability.
    //
    // Take note that a client address support at most one Host object
    // and attempts to create more should fail.
    //
    pub async fn create_host_on_network(&mut self) -> Result<Host, anyhow::Error> {
        let host_internal = self.netmgr.create_localhost_on_network().await?;
        Ok(Host {
            id: host_internal.object_id(),
            host_internal,
        })
    }

    // Ping Service
    //   JSON-RPC: Yes
    //   Gas Cost: Yes
    pub async fn ping_on_network(
        &mut self,
        target_host: &Host,
    ) -> Result<PingStats, anyhow::Error> {
        // Process with the Ping.
        self.netmgr
            .ping_on_network(&target_host.host_internal)
            .await
    }

    // Initialize Firewall Service
    //   JSON-RPC: Yes
    //   Gas Cost: Yes
    //
    // The firewall will be configureable from this point, but not yet enabled.
    pub async fn init_firewall(&mut self) -> Result<(), anyhow::Error> {
        self.netmgr.init_firewall().await
    }
}
