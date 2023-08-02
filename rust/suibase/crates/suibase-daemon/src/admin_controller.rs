use std::fmt::Debug;

use crate::basic_types::*;

use crate::network_monitor::NetMonTx;
use crate::proxy_server::ProxyServer;
use crate::shared_types::{Globals, InputPort, SafeWorkdirs, WorkdirProxyConfig, Workdirs};
use crate::workdirs_watcher::WorkdirsWatcher;

use anyhow::Result;

use tokio_graceful_shutdown::{FutureExt, NestedSubsystem, SubsystemHandle};

// Design
//
// The AdminController does:
//   - Process all system/configuration-level events that are easier to handle when done sequentially
//     (implemented by dequeing and processing one event at the time).
//   - Handle events to hot-reload the suibase.yaml
//   - Handle events for various user actions (e.g. from JSONRPCServer).
//   - Responsible to keep one "ProxyServer" per workdir running (localnet, devnet, testnet ...)
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
    idx: Option<ManagedVecUSize>,
    globals: Globals,
    admctrl_rx: AdminControllerRx,
    admctrl_tx: AdminControllerTx,
    netmon_tx: NetMonTx,
    workdirs: Workdirs,
    wd_tracking: AutoSizeVec<WorkdirTracking>,
    port_tracking: AutoSizeVec<InputPortTracking>,
}

pub type AdminControllerTx = tokio::sync::mpsc::Sender<AdminControllerMsg>;
pub type AdminControllerRx = tokio::sync::mpsc::Receiver<AdminControllerMsg>;

#[derive(Default, Debug)]
struct WorkdirTracking {
    last_read_config: Option<WorkdirProxyConfig>,
}

#[derive(Default)]
struct InputPortTracking {
    proxy_server_handle: Option<NestedSubsystem>, // Set when the proxy_server is started.
    port_number: u16, // port number used when the proxy_server was started.
}

pub struct AdminControllerMsg {
    // Message sent toward the AdminController from various sources.
    pub event_id: AdminControllerEventID,
    pub workdir_idx: Option<WorkdirIdx>,
    pub data_string: Option<String>,
    // Channel to send a one-time response.
    pub resp_channel: Option<tokio::sync::oneshot::Sender<String>>,
}

impl AdminControllerMsg {
    pub fn new() -> Self {
        Self {
            event_id: 0,
            workdir_idx: None,
            data_string: None,
            resp_channel: None,
        }
    }
    pub fn data_string(&self) -> Option<String> {
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
pub const EVENT_DEBUG_PRINT: u8 = 2;

impl AdminController {
    pub fn new(
        globals: Globals,
        workdirs: Workdirs,
        admctrl_rx: AdminControllerRx,
        admctrl_tx: AdminControllerTx,
        netmon_tx: NetMonTx,
    ) -> Self {
        Self {
            idx: None,
            globals,
            workdirs,
            admctrl_rx,
            admctrl_tx,
            netmon_tx,
            wd_tracking: AutoSizeVec::new(),   // WorkdirTracking
            port_tracking: AutoSizeVec::new(), // InputPortTracking
        }
    }

    async fn process_debug_print_msg(&mut self, msg: AdminControllerMsg) {
        // Send a response to the return channel with the debug print of a few
        // relevant internal states, particularly the configuration tracking.
        if msg.event_id != EVENT_DEBUG_PRINT {
            log::error!("Unexpected event_id {:?}", msg.event_id);
            // Do nothing. Consume the message.
            return;
        }
        if msg.resp_channel.is_none() {
            log::error!("EVENT_DEBUG_PRINT missing response channel");
            return;
        }
        let resp_channel = msg.resp_channel.unwrap();
        resp_channel
            .send(format!("\nwd_tracking: {:?}", self.wd_tracking))
            .unwrap();
    }

    async fn process_config_msg(&mut self, msg: AdminControllerMsg, subsys: &SubsystemHandle) {
        // This process always process only one Workdir at a time.

        if msg.event_id != EVENT_NOTIF_CONFIG_FILE_CHANGE {
            log::error!("Unexpected event_id {:?}", msg.event_id);
            // Do nothing. Consume the message.
            return;
        }

        if msg.data_string().is_none() {
            log::error!("EVENT_NOTIF_CONFIG_FILE_CHANGE missing path information");
            return;
        }
        let path = msg.data_string().unwrap();

        // Here will be done the operation that requires a write lock on the workdirs
        // (none for now, the workdir are hardcoded).

        // Load the configuration.
        let mut workdir_config = WorkdirProxyConfig::new();
        let workdir_idx: u8;
        let workdir_name: String;
        {
            let workdirs_guard = self.workdirs.read().await;
            let workdirs = &*workdirs_guard;

            let workdir_search_result = workdirs.find_workdir(&path);
            if workdir_search_result.is_none() {
                log::error!("Workdir not found for path {:?}", &msg.data_string());
                // Do nothing. Consume the message.
                return;
            }
            let (found_workdir_idx, workdir) = workdir_search_result.unwrap();
            workdir_idx = found_workdir_idx;
            workdir_name = workdir.name().to_string();

            // Load the 3xsuibase.yaml. The default, common and user version in order.
            let try_load = workdir_config
                .load_and_merge_from_file(&workdir.suibase_yaml_default().to_string_lossy());
            if try_load.is_err() {
                log::error!(
                    "Failed to load default config file {:?}",
                    workdir.suibase_yaml_default()
                );
                // Do nothing. Consume the message.
                return;
            }

            // Optional, so no error if does not exists.
            let _ = workdir_config
                .load_and_merge_from_common_file(&workdirs.suibase_yaml_common().to_string_lossy());

            let _ = workdir_config
                .load_and_merge_from_file(&workdir.suibase_yaml_user().to_string_lossy());

            let _ = workdir_config.load_state_file(&workdir.suibase_state_file().to_string_lossy());
        } // Release Workdirs read lock

        // Check if workdir_config has changed since last_read_config.
        let wd_tracking = self.wd_tracking.get_mut(workdir_idx);

        if wd_tracking.last_read_config.is_some() {
            let last_read_config = wd_tracking.last_read_config.as_ref().unwrap();
            log::debug!(
                "cfg user_request last_read {:?} current {:?} ",
                last_read_config.user_request(),
                workdir_config.user_request()
            );
            if last_read_config == &workdir_config {
                log::debug!("cfg notif {} no change", workdir_name);
                // Do nothing. Consume the message.
                return;
            }
        }

        log::info!("cfg notif {}", workdir_name);

        // Apply the configuration to the globals.
        let config_applied: Option<(ManagedVecUSize, u16)> = {
            // Get a write lock on the globals.
            let mut globals_guard = self.globals.write().await;
            let globals = &mut *globals_guard;

            // Apply the config to add/modify the related InputPort in the globals (as needed).
            //
            // Default listening ports
            //    44343 (mainnet RPC)
            //    44342 (testnet RPC)
            //    44341 (devnet RPC)
            //    44340 (localnet RPC)
            let ports = &mut globals.input_ports;

            // Find the InputPort with a matching workdir_idx.
            let input_port_search = ports.iter_mut().find(|p| p.1.workdir_idx() == workdir_idx);
            if let Some((port_idx, input_port)) = input_port_search {
                if input_port.is_proxy_enabled() != workdir_config.is_proxy_enabled() {
                    input_port.set_proxy_enabled(workdir_config.is_proxy_enabled());
                }
                if input_port.is_user_request_start() != workdir_config.is_user_request_start() {
                    input_port.set_user_request_start(workdir_config.is_user_request_start());
                }
                Some((port_idx, input_port.port_number()))
            } else {
                // TODO Verify there is no conflicting port assigment.
                let mut input_port =
                    InputPort::new(workdir_idx, workdir_name.clone(), &workdir_config);
                for (key, value) in workdir_config.links().iter() {
                    if let Some(rpc) = &value.rpc {
                        input_port.add_target_server(rpc.clone(), key.clone());
                    }
                }
                let port_number = input_port.port_number();
                ports
                    .push(input_port)
                    .map(|port_idx| (port_idx, port_number))
            }
        }; // Release Globals write lock

        if let Some((port_idx, port_number)) = config_applied {
            // As needed, start a proxy server for this port.
            let port_tracking = self.port_tracking.get_mut(port_idx);

            if port_tracking.proxy_server_handle.is_none() {
                let proxy_server = ProxyServer::new();
                let globals = self.globals.clone();
                let netmon_tx = self.netmon_tx.clone();
                port_tracking.proxy_server_handle = Some(subsys.start("proxy-server", move |a| {
                    proxy_server.run(a, port_idx, globals, netmon_tx)
                }));
                port_tracking.port_number = port_number;
            } else {
                // Monitor a port number change. This is a rare "fundamental" configuration change that
                // is simpler to handle by exiting the process (and let it be restarted automatically
                // by its parent suibase script). The alternative would be to stop the TCP listening
                // thread and coordinate with all other supporting threads only for ONE port and that is
                // feasible but challenging to get right... particularly if the user does weird stuff like
                // quickly toggling ports assignment between two workdirs (!!!).
                if port_number != port_tracking.port_number {
                    log::info!(
                        "Port number changed from {} to {}",
                        port_tracking.port_number,
                        port_number
                    );
                    // Sleep a bit in case of a "restart loop" bug.
                    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    subsys.request_shutdown();
                }
            }
        }

        // Remember the changes that were applied.
        wd_tracking.last_read_config = Some(workdir_config);
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = self.admctrl_rx.recv().await {
                match msg.event_id {
                    EVENT_DEBUG_PRINT => {
                        self.process_debug_print_msg(msg).await;
                    }
                    EVENT_NOTIF_CONFIG_FILE_CHANGE => {
                        self.process_config_msg(msg, subsys).await;
                    }
                    _ => {
                        log::error!("Unknown event_id {}", msg.event_id);
                    }
                }
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        // This is the "master" thread that controls the changes to the
        // configuration. It is responsible to start/stop other subsystems.
        log::info!("started");

        // Initialize a subsystem to watch workdirs files. Notifications are then
        // send back to this thread on the AdminController channel.
        {
            let admctrl_tx = self.admctrl_tx.clone();
            let workdirs_watcher = WorkdirsWatcher::new(self.workdirs.clone(), admctrl_tx);
            subsys.start("workdirs-watcher", move |a| workdirs_watcher.run(a));
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

impl ManagedElement for AdminController {
    fn idx(&self) -> Option<ManagedVecUSize> {
        self.idx
    }
    fn set_idx(&mut self, idx: Option<ManagedVecUSize>) {
        self.idx = idx;
    }
}

#[test]
fn test_load_config_from_suibase_default() {
    // Note: More of a functional test. Suibase need to be installed.

    // Test a known "standard" localnet suibase.yaml
    let workdirs = SafeWorkdirs::new();
    let mut path = std::path::PathBuf::from(workdirs.suibase_home());
    path.push("scripts");
    path.push("defaults");
    path.push("localnet");
    path.push("suibase.yaml");

    let workdir_search_result = workdirs.find_workdir(&path.to_string_lossy().to_string());
    assert!(workdir_search_result.is_some());
    let (_workdir_idx, workdir) = workdir_search_result.unwrap();

    let mut config = WorkdirProxyConfig::new();
    let result = config.load_and_merge_from_file(
        &workdir
            .suibase_yaml_default()
            .to_string_lossy()
            .to_string()
            .to_string(),
    );
    assert!(result.is_ok());
    // Expected:
    // - alias: "localnet"
    //   rpc: "http://0.0.0.0:9000"
    //   ws: "ws://0.0.0.0:9000"
    assert_eq!(config.links_overrides(), false);
    assert_eq!(config.links().len(), 1);
    assert!(config.links().contains_key("localnet"));
    assert!(config.links().get("localnet").unwrap().rpc.is_some());
    assert!(config.links().get("localnet").unwrap().ws.is_some());
    let link = config.links().get("localnet").unwrap();
    assert_eq!(link.rpc.as_ref().unwrap(), "http://0.0.0.0:9000");
    assert_eq!(link.ws.as_ref().unwrap(), "ws://0.0.0.0:9000");
}
