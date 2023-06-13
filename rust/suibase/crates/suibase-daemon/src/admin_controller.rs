use std::collections::HashMap;

use crate::basic_types::*;
use crate::globals::{Globals, PortStates};
use crate::network_monitor::NetMonTx;
use crate::proxy_server::ProxyServer;
use crate::target_server::TargetServer;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use tokio_graceful_shutdown::SubsystemHandle;

struct ProxyServerManagement {
    created_time: EpochTimestamp,
}

type ProxyServers = HashMap<PortMapID, ProxyServerManagement>;

pub struct AdminController {
    globals: Globals,
    netmon_tx: NetMonTx,
    proxy_servers: ProxyServers,
}

impl AdminController {
    pub fn new(globals: Globals, netmon_tx: NetMonTx) -> Self {
        Self {
            globals,
            netmon_tx,
            proxy_servers: ProxyServers::new(),
        }
    }

    pub async fn load_config(&mut self, subsys: &SubsystemHandle) {
        // TODO Check first if there is something to load... before getting a write lock.

        // Get a write lock on the globals.
        let mut globals_guard = self.globals.write().await;
        let globals = &mut *globals_guard;

        // Add default listen ports
        //    44343 (mainnet RPC)
        //    44342 (testnet RPC)
        //    44341 (devnet RPC)
        //    44340 (localnet RPC)
        let ports = &mut globals.input_ports;

        if ports.map.len() == 0 {
            // Add target server vincagame and Mysten labs.
            let mut port_states = PortStates::new(9124);
            let id = port_states.id();

            // Add sui-rpc-mainnet.testnet-pride.com:443
            //port_states.target_servers.insert( TargetServer::new("142.132.202.87:443") );
            let connection_str = "0.0.0.0:9123";
            port_states.target_servers.insert(
                connection_str.to_string(),
                TargetServer::new(connection_str),
            );

            // Add it to globals.
            ports.map.insert(id, port_states);

            // TODO: Try to leave the globals lock here...

            // Start a proxy server for this port.
            let proxy_server = ProxyServer::new(self.globals.clone(), self.netmon_tx.clone());

            // Remember the proxy server has been started for this port.
            self.proxy_servers.insert(
                id,
                ProxyServerManagement {
                    created_time: EpochTimestamp::now(),
                },
            );

            subsys.start("proxy_server", move |a| proxy_server.run(a, id));
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        // This is going to be the API thread later... for now just "load" the config.
        log::info!("started");

        self.load_config(&subsys).await;

        loop {
            sleep(Duration::from_millis(1000)).await;
            if subsys.is_shutdown_requested() {
                // Do a normal shutdown.
                log::info!("shutting down (2)");
                return Ok(());
            }
        }

        // Task ends with an error. This should cause the main program to shutdown.
        // Err(anyhow!("AdminController threw an error."))

        // Normal shutdown:
        //   subsys.on_shutdown_requested().await;
        //   log::info!("Shutting down Subsystem1 ...");
        //   log::info!("Subsystem1 stopped.");
        //   Ok(())
    }
}
