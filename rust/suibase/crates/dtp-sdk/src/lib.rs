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

use std::{str::FromStr, sync::Arc};

use anyhow::bail;
use dtp_core::{
    network::{
        HostInternalMT, HostInternalST, NetworkManagerMT, NetworkManagerST,
        TransportControlInternalMT,
    },
    types::PingStats,
};
use log::info;
use sui_sdk::types::base_types::{ObjectID, SuiAddress};

// Re-export ConnObjectsInternal for debug purposes.
pub use dtp_core::network::ConnObjectsInternal;

#[derive(Debug, Clone)]
pub struct Host {
    // Host can be cheaply cloned and safely sent/shared between multiple threads.
    //
    // User can have multiple Host object instance. Instances with the same ObjectID
    // share the same internal data (reference counted with Arc::Mutex).

    // Convenient read-only vars that do not change for the lifetime of this object.
    id: ObjectID,

    // Multi-thread safe implementation hidden in dtp-core.
    host_internal: HostInternalMT,
}

impl Host {
    pub fn object_id(&self) -> &ObjectID {
        &self.id
    }
}

#[derive(Debug)]
pub struct Connection {
    // Multi-thread safe implementation hidden in dtp-core.
    tc_internal: TransportControlInternalMT,
}

impl Connection {
    pub async fn get_conn_objects(&self) -> Option<ConnObjectsInternal> {
        let tc_guard = self.tc_internal.read().await;
        let tc = &*tc_guard;
        tc.get_conn_objects()
    }
}

#[derive(Debug)]
pub struct DTP {
    // Multi-thread safe implementation hidden in dtp-core.
    netmgr: NetworkManagerMT,
}

impl DTP {
    pub async fn new(
        auth_address: SuiAddress,
        keystore_pathname: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        let netmgr = Arc::new(tokio::sync::RwLock::new(
            NetworkManagerST::new(auth_address, keystore_pathname).await?,
        ));
        Ok(DTP {
            #[allow(clippy::needless_borrow)]
            netmgr,
        })
    }

    // Mutators
    //   JSON-RPC: No
    //   Gas Cost: No
    // TODO Refactor into a DTP builder to avoid error prone need for caller to do "await".
    pub async fn set_package_id(&mut self, package_id: ObjectID) {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        netmgr.set_package_id(package_id);
    }

    pub async fn set_gas_address(&mut self, gas_address: SuiAddress) {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        netmgr.set_gas_address(gas_address);
    }

    // Mutators
    //   JSON-RPC: Sometimes
    //   Gas Cost: No
    pub async fn add_rpc_url(&mut self, http_url: &str) -> Result<(), anyhow::Error> {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        netmgr.add_rpc_url(http_url).await
    }

    // Accessors
    //   JSON-RPC: No
    //   Gas Cost: No
    pub async fn package_id(&self) -> ObjectID {
        let netmgr_guard = self.netmgr.write().await;
        let netmgr = &*netmgr_guard;

        *netmgr.get_package_id()
    }

    pub async fn client_address(&self) -> SuiAddress {
        let netmgr_guard = self.netmgr.write().await;
        let netmgr = &*netmgr_guard;

        *netmgr.get_auth_address()
    }

    pub async fn gas_address(&self) -> SuiAddress {
        let netmgr_guard = self.netmgr.write().await;
        let netmgr = &*netmgr_guard;

        *netmgr.get_gas_address()
    }

    pub async fn localhost_id(&self) -> Option<ObjectID> {
        let netmgr_guard = self.netmgr.write().await;
        let netmgr = &*netmgr_guard;

        *netmgr.get_localhost_id()
    }

    // get_host
    //   JSON-RPC: Yes
    //   Gas Cost: Yes
    //
    // Get the Host for the auth address (specified at DTP instance creation).
    //
    // On success it means the caller have administrative capability on that Host object.
    // (can setup firewall, enable services etc...)
    //
    // If the host does not exists, it will be tentatively created on the network.
    pub async fn get_host(&mut self) -> Result<Host, anyhow::Error> {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        info!("get_host start");
        // Note: the netmgr do also cache the LocalhostInternal. Can it be used?
        // For now, always retrieve latest from network.
        let mut host_internal: Option<HostInternalST> = None;
        if netmgr.get_localhost_id().is_none() {
            info!("get_host A");
            // Best-effort find among owned object of auth.
            let result = netmgr.get_localhost_by_auth().await?;
            if let Some(result) = result {
                host_internal = Some(result);
                info!("get_host B");
            }
        } else {
            // Get latest from likely existing Host object on the network.
            let localhost_id = netmgr.get_localhost_id().unwrap();
            host_internal = netmgr.get_host_by_id(localhost_id).await?;
            info!("get_host C");
        }
        if host_internal.is_none() {
            // Create a new Host object on the network.
            host_internal = Some(netmgr.create_localhost_on_network().await?);
            info!("get_host D");
        }
        // Should exist at this point.
        let host_internal = host_internal.unwrap();

        netmgr.sync_registry().await?;

        info!("get_host end");
        Ok(Host {
            id: host_internal.object_id(),
            host_internal: Arc::new(tokio::sync::RwLock::new(host_internal)),
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
    //
    // Returns Ok(None) if confirmed that the host does not exists.
    pub async fn get_host_by_id(&self, host_id: ObjectID) -> Result<Option<Host>, anyhow::Error> {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        let host_internal = netmgr.get_host_by_id(host_id).await?;
        if host_internal.is_none() {
            return Ok(None);
        }
        let host_internal = host_internal.unwrap();
        Ok(Some(Host {
            id: host_internal.object_id(),
            host_internal: Arc::new(tokio::sync::RwLock::new(host_internal)),
        }))
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
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        let host_internal = netmgr.create_localhost_on_network().await?;
        Ok(Host {
            id: host_internal.object_id(),
            host_internal: Arc::new(tokio::sync::RwLock::new(host_internal)),
        })
    }

    // Ping Service
    //   JSON-RPC: Yes
    //   Gas Cost: Yes
    //
    // Note: This util fn not yet implemented. For now, use create_connection()/send()
    pub async fn ping_on_network(
        &mut self,
        target_host: &Host,
    ) -> Result<PingStats, anyhow::Error> {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        let target_host_guard = target_host.host_internal.read().await;
        let target_host_internal = &*target_host_guard;

        // Process with the Ping.
        netmgr.ping_on_network(&target_host_internal).await
    }

    // Create a connection to a Host.
    //   JSON-RPC: Yes
    //   Gas Cost: Yes
    //
    // Use target Host object and ServiceType.
    pub async fn create_connection(
        &mut self,
        target_host: &Host,
        service_idx: u8,
    ) -> Result<Connection, anyhow::Error> {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        let target_host_guard = target_host.host_internal.read().await;
        let target_host_internal = &*target_host_guard;

        Ok(Connection {
            tc_internal: netmgr
                .create_connection(&target_host_internal, service_idx)
                .await?,
        })
    }

    // Initialize Firewall Service
    //   JSON-RPC: Yes
    //   Gas Cost: Yes
    //
    // The firewall will be configureable from this point, but not yet enabled.
    pub async fn init_firewall(&mut self) -> Result<(), anyhow::Error> {
        let mut netmgr_guard = self.netmgr.write().await;
        let netmgr = &mut *netmgr_guard;

        netmgr.init_firewall().await
    }
}

// Utility functions.
pub fn str_to_sui_address(address: &str) -> Result<SuiAddress, anyhow::Error> {
    // If address does not start with "0x", append it to address.
    // Can you please code this?
    let address = if address.starts_with("0x") {
        address.to_string()
    } else {
        format!("0x{}", address)
    };

    let ret_value = SuiAddress::from_str(&address);
    if let Err(e) = ret_value {
        bail!("address invalid: {} {}", address, e.to_string())
    }
    Ok(ret_value.unwrap())
}

pub fn str_to_object_id(object_id: &str) -> Result<ObjectID, anyhow::Error> {
    // If object_id does not start with "0x", append it to object_id.
    let object_id = if object_id.starts_with("0x") {
        object_id.to_string()
    } else {
        format!("0x{}", object_id)
    };

    let ret_value = ObjectID::from_str(&object_id);
    if let Err(e) = ret_value {
        bail!("object id invalid: {} {}", object_id, e.to_string())
    }
    Ok(ret_value.unwrap())
}
