use crate::basic_types::*;
use crate::globals::Globals;
use crate::input_port::InputPort;
use crate::network_monitor::NetMonTx;
use crate::proxy_server::ProxyServer;
use crate::target_server::TargetServer;
use crate::workdirs::Workdirs;

use anyhow::Result;

use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use std::{collections::HashMap, path::Path, path::PathBuf, str::FromStr};

// Design (WIP)
//
// Use configuration are hot-reloaded from the suibase.yaml into the Globals (RwLock).
//
// The AdminController will:
//   - Hot-reload the Globals from the suibase.yaml (after proper validation and RwLock).
//   - Start one "ProxyServer" per workdir (localnet, devnet, testnet ...)
//   - Serve the JSON-RPC API.
//
// Globals: InputPort Instantiation
// ================================
// One InputPort is instantiated per workdir (localnet, devnet, testnet ...).
//
// Once instantiated, it is never deleted. Subsequently, the ProxyServer is also started
// and never stopped. It can be disabled/re-enabled though.
//
// The ProxyServer function can be disabled at workdir granularity by the user config and/or
// if the workdir is deleted.

pub struct AdminController {
    globals: Globals,
    admctrl_rx: AdminControllerRx,
    admctrl_tx: AdminControllerTx,
    netmon_tx: NetMonTx,
    workdirs: Workdirs,
}

pub type AdminControllerTx = tokio::sync::mpsc::Sender<AdminControllerMsg>;
pub type AdminControllerRx = tokio::sync::mpsc::Receiver<AdminControllerMsg>;

pub struct AdminControllerMsg {
    // Message sent toward the AdminController from various sources.
    event_id: AdminControllerEventID,
    data_string: String,
}

impl AdminControllerMsg {
    pub fn new() -> Self {
        Self {
            event_id: 0,
            data_string: String::new(),
        }
    }
    pub fn data_string(&self) -> String {
        self.data_string.clone()
    }
}

impl std::fmt::Debug for AdminControllerMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminControllerMsg")
            .field("event_id", &self.event_id)
            .field("data_string", &self.data_string)
            .finish()
    }
}

// Events ID
pub type AdminControllerEventID = u8;
pub const EVENT_NOTIF_CONFIG_FILE_CHANGE: u8 = 1;

struct Link {
    alias: String,
    enabled: bool,
    rpc: Option<String>,
    ws: Option<String>,
    priority: u8,
}
struct LinksConfig {
    links: HashMap<String, Link>, // alias is also the key. TODO Look into Hashset?
    links_overrides: bool,
}

impl LinksConfig {
    pub fn new() -> Self {
        Self {
            links: HashMap::new(),
            links_overrides: false,
        }
    }

    pub fn links_overrides(&self) -> bool {
        self.links_overrides
    }

    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        // Example of suibase.yaml:
        //
        // links:
        //  - alias: "localnet"
        //    rpc: "http://0.0.0.0:9000"
        //    ws: "ws://0.0.0.0:9000"
        //    priority: 1
        //  - alias: "localnet"
        //    rpc: "http://0.0.0.0:9000"
        //    ws: "ws://0.0.0.0:9000"
        //    priority: 2
        let contents = std::fs::read_to_string(path)?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)?;
        // TODO: Implement links override flag.
        if let Some(links) = yaml["links"].as_sequence() {
            for link in links {
                // TODO: Lots of robustness needed to be added here...
                if let Some(alias) = link["alias"].as_str() {
                    // Purpose of "enabled" is actually to disable a link... so if not present, default
                    // to enabled.
                    let enabled = link["enabled"].as_bool().unwrap_or_else(|| true);
                    let rpc = link["rpc"].as_str().map(|s| s.to_string()); // Optional
                    let ws = link["ws"].as_str().map(|s| s.to_string()); // Optional
                                                                         // Should instead remove all alpha, do absolute value, and clamp to 1-255.
                    let priority = link["priority"].as_u64().unwrap_or_else(|| u64::MAX) as u8;
                    let link = Link {
                        alias: alias.to_string(),
                        enabled,
                        rpc,
                        ws,
                        priority,
                    };

                    self.links.insert(alias.to_string(), link);
                }
            }
        }

        Ok(())
    }
}

impl AdminController {
    pub fn new(
        globals: Globals,
        admctrl_rx: AdminControllerRx,
        admctrl_tx: AdminControllerTx,
        netmon_tx: NetMonTx,
    ) -> Self {
        Self {
            globals,
            admctrl_rx,
            admctrl_tx,
            netmon_tx,
            workdirs: Workdirs::new(),
        }
    }

    async fn send_notif_config_file_change(&mut self, path: String) {
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_NOTIF_CONFIG_FILE_CHANGE;
        msg.data_string = path;
        let _ = self.admctrl_tx.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
        });
    }

    async fn process_config_msg(&mut self, msg: AdminControllerMsg, subsys: &SubsystemHandle) {
        // TODO Check first if there is something to load... before getting a write lock.
        log::info!("Processing config file change notification {:?}", msg);

        let workdir = self.workdirs.find_workdir(&msg.data_string());
        if workdir.is_none() {
            log::error!("Workdir not found for path {:?}", &msg.data_string());
            // Do nothing. Consume the message.
            return;
        }
        let workdir = workdir.unwrap();

        // TODO Load the user config (if exists).
        // TODO Load the default (unless the user config completely overides it).
        // For now, just load the default.
        let config_candidate = LinksConfig::new().load_from_file(workdir.suibase_yaml_default());

        // Get a write lock on the globals.
        let port_id: InputPortIdx = {
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
                let mut input_port = InputPort::new(44343);
                let mut uri = "https://sui-rpc-mainnet.testnet-pride.com:443";
                input_port
                    .target_servers
                    .push(TargetServer::new(uri.to_string()));

                uri = "https://fullnode.mainnet.sui.io:443";
                input_port
                    .target_servers
                    .push(TargetServer::new(uri.to_string()));

                // TODO Rework this for error handling.
                if let Some(port_id) = ports.map.push(input_port) {
                    port_id
                } else {
                    ManagedVecUSize::MAX
                }
            } else {
                ManagedVecUSize::MAX
            }
        }; // Release Globals write lock

        if port_id != ManagedVecUSize::MAX {
            // Start a proxy server for this port.
            let proxy_server = ProxyServer::new();
            let globals = self.globals.clone();
            let netmon_tx = self.netmon_tx.clone();
            subsys.start("proxy_server", move |a| {
                proxy_server.run(a, port_id, globals, netmon_tx)
            });
        }
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = self.admctrl_rx.recv().await {
                // Process the message.
                self.process_config_msg(msg, &subsys).await;
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        // This is going to be the API thread later... for now just "load" the config.
        log::info!("started");

        // This is the point where the user configuration can be loaded into
        // the globals. Do not rely on the file watcher, instead prime the
        // event into the queue to force the config to be loaded right now.
        let workdirs = Workdirs::new();
        for workdir in workdirs.workdirs {
            self.send_notif_config_file_change(workdir.1.suibase_yaml_default().to_string())
                .await;
        }

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("shutting down - normal exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("shutting down - normal exit (1)");
                Ok(())
            }
        }
    }
}

#[test]
fn test_load_config_from_suibase_default() {
    // Note: More of a functional test. Suibase need to be installed.

    // Test a known "standard" localnet suibase.yaml
    let workdirs = Workdirs::new();
    let mut path = PathBuf::from(workdirs.suibase_home());
    path.push("scripts");
    path.push("defaults");
    path.push("localnet");
    path.push("suibase.yaml");

    let workdir = workdirs.find_workdir(&path.to_string_lossy().to_string());
    assert!(workdir.is_some());
    let workdir = workdir.unwrap();

    let mut config = LinksConfig::new();
    let result = config.load_from_file(workdir.suibase_yaml_default());
    assert!(result.is_ok());
    // Expected:
    // - alias: "localnet"
    //   rpc: "http://0.0.0.0:9000"
    //   ws: "ws://0.0.0.0:9000"
    assert_eq!(config.links_overrides(), false);
    assert_eq!(config.links.len(), 1);
    assert!(config.links.contains_key("localnet"));
    assert!(config.links.get("localnet").unwrap().rpc.is_some());
    assert!(config.links.get("localnet").unwrap().ws.is_some());
    let link = config.links.get("localnet").unwrap();
    assert_eq!(link.rpc.as_ref().unwrap(), "http://0.0.0.0:9000");
    assert_eq!(link.ws.as_ref().unwrap(), "ws://0.0.0.0:9000");
}
