use std::collections::HashMap;

use crate::basic_types::*;
use crate::globals::Globals;
use crate::network_monitor::NetMonTx;
use crate::port_states::PortStates;
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
        let port_id = {
            let mut globals_guard = self.globals.write().await;
            let globals = &mut *globals_guard;

            // Add default listening ports
            //    44343 (mainnet RPC)
            //    44342 (testnet RPC)
            //    44341 (devnet RPC)
            //    44340 (localnet RPC)
            let ports = &mut globals.input_ports;

            if ports.map.len() == 0 {
                // Add target servers
                let mut port_states = PortStates::new(44343);
                let port_id = port_states.id();
                let mut uri = "https://sui-rpc-mainnet.testnet-pride.com:443";
                port_states
                    .target_servers
                    .insert(uri.to_string(), TargetServer::new(uri.to_string()));

                uri = "https://fullnode.mainnet.sui.io:443";
                port_states
                    .target_servers
                    .insert(uri.to_string(), TargetServer::new(uri.to_string()));

                // Add it to globals.
                ports.map.insert(port_id, port_states);

                port_id
            } else {
                0
            }
        }; // Release Globals write lock

        if port_id != 0 {
            // Start a proxy server for this port.
            let proxy_server = ProxyServer::new();
            let globals = self.globals.clone();
            let netmon_tx = self.netmon_tx.clone();
            subsys.start("proxy_server", move |a| {
                proxy_server.run(a, port_id, globals, netmon_tx)
            });
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
