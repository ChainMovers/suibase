use std::error::Error;
use std::time::Duration;

use common::shared_types::WORKDIRS_KEYS;
use common::{basic_types::*, log_safe};

use crate::network_monitor::NetMonTx;
use crate::proxy_server::ProxyServer;
use crate::shared_types::{Globals, InputPort, WorkdirUserConfig, WORKDIR_IDX_LOCALNET};
use crate::workdirs_watcher::WorkdirsWatcher;
use crate::workers::{
    CliPoller, CliPollerParams, EventsWriterWorker, EventsWriterWorkerParams, PackagesPoller,
    PackagesPollerParams,
};
use common::workers::ShellWorker;

use anyhow::{anyhow, Result};

use tokio_graceful_shutdown::{FutureExt, NestedSubsystem, SubsystemBuilder, SubsystemHandle};

// Design
//
// The AdminController does:
//   - Process all system/configuration-level events that are easier to handle when done sequentially
//     (implemented by dequeuing and processing one event at the time).
//   - Handle events to hot-reload the suibase.yaml
//   - Handle events for various user actions (e.g. from JSONRPCServer).
//   - Responsible to keep one "ProxyServer" and "ShellProcessor" running per workdir.
//
// globals.proxy: InputPort Instantiation
// =======================================
// One InputPort is instantiated per workdir (localnet, devnet, testnet ...).
//
// Once instantiated, it is never deleted. Subsequently, the ProxyServer is also started
// and never stopped. It can be disabled/re-enabled though.
//
// The ProxyServer function can be disabled at workdir granularity by the user config and/or
// if the workdir is deleted.

pub struct AdminController {
    idx: Option<ManagedVecU8>,
    globals: Globals,

    admctrl_rx: AdminControllerRx,
    admctrl_tx: AdminControllerTx,
    netmon_tx: NetMonTx,

    wd_tracking: AutoSizeVec<WorkdirTracking>,
    port_tracking: AutoSizeVec<InputPortTracking>,
}

#[derive(Default)]
struct WorkdirTracking {
    last_read_config: Option<WorkdirUserConfig>,

    shell_worker_tx: Option<GenericTx>,
    shell_worker_handle: Option<NestedSubsystem<Box<dyn Error + Send + Sync>>>, // Set when the ShellWorker is started.

    events_worker_tx: Option<GenericTx>,
    events_worker_handle: Option<NestedSubsystem<Box<dyn Error + Send + Sync>>>, // Set when the EventsWriterWorker is started.

    cli_poller: Option<CliPoller>,

    packages_poller: Option<PackagesPoller>,

    process_watchdog_last_check_timestamp: Option<tokio::time::Instant>,
    process_watchdog_last_recovery_timestamp: Option<tokio::time::Instant>,
}

impl std::fmt::Debug for WorkdirTracking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkdirTracking")
            // NestedSubsystem does not implement Debug
            .field("last_read_config", &self.last_read_config)
            .finish()
    }
}

#[derive(Default)]
struct InputPortTracking {
    proxy_server_handle: Option<NestedSubsystem<Box<dyn Error + Send + Sync>>>, // Set when the proxy_server is started.
    port_number: u16, // port number used when the proxy_server was started.
}

impl std::fmt::Debug for InputPortTracking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkdirTracking")
            // NestedSubsystem does not implement Debug
            .field("port_number", &self.port_number)
            .finish()
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
            idx: None,
            globals,
            admctrl_rx,
            admctrl_tx,
            netmon_tx,
            wd_tracking: AutoSizeVec::new(),   // WorkdirTracking
            port_tracking: AutoSizeVec::new(), // InputPortTracking
        }
    }

    // Use the send_XXXXXX functions to queue a message to the AdminController.
    pub async fn send_event_audit(tx_channel: &AdminControllerTx) -> Result<()> {
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_AUDIT;
        if let Err(e) = tx_channel.try_send(msg) {
            let err_msg = format!("send_event_audit: {}", e);
            log_safe!(err_msg);
            return Err(anyhow!(err_msg));
        }
        Ok(())
    }

    pub async fn send_event_update(
        tx_channel: &AdminControllerTx,
        workdir_idx: WorkdirIdx,
    ) -> Result<()> {
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_UPDATE;
        msg.workdir_idx = Some(workdir_idx);
        if let Err(e) = tx_channel.try_send(msg) {
            let err_msg = format!("send_event_update: {}", e);
            log_safe!(err_msg);
            return Err(anyhow!(err_msg));
        }
        Ok(())
    }

    pub async fn send_shell_exec(
        tx_channel: &AdminControllerTx,
        workdir_idx: WorkdirIdx,
        cmd: String,
    ) -> Result<String> {
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_SHELL_EXEC;
        let (tx, rx) = tokio::sync::oneshot::channel();
        msg.resp_channel = Some(tx);
        msg.workdir_idx = Some(workdir_idx);
        msg.data_string = Some(cmd.clone());
        // The purpose of the timeout is the error log to help debugging
        // if the shell call is apparently "stuck".
        const TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour!
        if (tx_channel.send(msg).await).is_ok() {
            match tokio::time::timeout(TIMEOUT, rx).await {
                Ok(Ok(resp_str)) => {
                    return Ok(resp_str);
                }
                Ok(Err(e)) => {
                    return Err(anyhow!("send_shell_exec internal error: {}", e.to_string()));
                }
                Err(_) => {
                    let timeout_err = format!("send_shell_exec timeout {}", cmd);
                    log::error!("{}", timeout_err);
                    return Err(anyhow!(timeout_err));
                }
            }
        }
        Err(anyhow!("send_shell_exec failed"))
    }

    async fn process_audit_msg(&mut self, msg: AdminControllerMsg) {
        if msg.event_id != EVENT_AUDIT {
            log::error!("Unexpected event_id {:?}", msg.event_id);
            // Do nothing. Consume the message.
            return;
        }

        // Forward an audit message to various "per workdir" worker(s).
        let mut worker_msg = GenericChannelMsg::new();
        worker_msg.event_id = EVENT_AUDIT;

        for (workdir_idx, wd_tracking) in self.wd_tracking.iter_mut() {
            worker_msg.workdir_idx = Some(workdir_idx);

            if let Some(worker_tx) = wd_tracking.events_worker_tx.as_ref() {
                match worker_tx.try_send(worker_msg.clone()) {
                    Ok(()) => {}
                    Err(e) => {
                        log_safe!(format!(
                            "try_send EVENT_AUDIT forward to events worker failed: {}",
                            e
                        ));
                    }
                }
            }

            Self::send_msg_to_cli_poller(wd_tracking, worker_msg.clone()).await;
            Self::send_msg_to_packages_poller(wd_tracking, worker_msg.clone()).await;
        }

        // Check for potential need for local process restart/recovery.
        self.watchdog_local_processes().await;
    }

    async fn send_msg_to_cli_poller(wd_tracking: &WorkdirTracking, msg: GenericChannelMsg) {
        if let Some(poller) = wd_tracking.cli_poller.as_ref() {
            let workdir_idx = msg.workdir_idx;
            let event_id = msg.event_id;
            match poller.get_tx_channel().try_send(msg) {
                Ok(()) => {}
                Err(e) => {
                    log_safe!(format!(
                        "try_send event id={:?} to {:?} cli poller failed: {}",
                        event_id, workdir_idx, e
                    ));
                }
            }
        }
    }

    async fn send_msg_to_packages_poller(wd_tracking: &WorkdirTracking, msg: GenericChannelMsg) {
        if let Some(poller) = wd_tracking.packages_poller.as_ref() {
            let workdir_idx = msg.workdir_idx;
            let event_id = msg.event_id;
            match poller.get_tx_channel().try_send(msg) {
                Ok(()) => {}
                Err(e) => {
                    log_safe!(format!(
                        "try_send event id={:?} to {:?} packages poller failed: {}",
                        event_id, workdir_idx, e
                    ));
                }
            }
        }
    }

    async fn process_update_msg(&mut self, msg: AdminControllerMsg) {
        if msg.event_id != EVENT_UPDATE {
            log::error!("Unexpected event_id {:?}", msg.event_id);
            // Do nothing. Consume the message.
            return;
        }

        // If workdir_idx is specified, forward an update message to
        // related workdir worker(s). If not specified, then
        // forward to all workdir worker(s).

        // Forward an update message to various "per workdir" worker(s).
        let mut worker_msg = GenericChannelMsg::new();
        worker_msg.event_id = EVENT_UPDATE;

        if let Some(workdir_idx) = msg.workdir_idx {
            if let Some(wd_tracking) = self.wd_tracking.get_if_some(workdir_idx) {
                worker_msg.workdir_idx = Some(workdir_idx);
                Self::send_msg_to_cli_poller(wd_tracking, worker_msg.clone()).await;
                Self::send_msg_to_packages_poller(wd_tracking, worker_msg.clone()).await;
            }
        } else {
            for (workdir_idx, wd_tracking) in self.wd_tracking.iter() {
                worker_msg.workdir_idx = Some(workdir_idx);
                Self::send_msg_to_cli_poller(wd_tracking, worker_msg.clone()).await;
                Self::send_msg_to_packages_poller(wd_tracking, worker_msg.clone()).await;
            }
        }
    }

    async fn watchdog_local_processes(&mut self) {
        let (last_check_timestamp, last_recovery_timestamp) = {
            if let Some(workdir_tracking) = self.wd_tracking.get_if_some(WORKDIR_IDX_LOCALNET) {
                (
                    workdir_tracking.process_watchdog_last_check_timestamp,
                    workdir_tracking.process_watchdog_last_recovery_timestamp,
                )
            } else {
                return;
            }
        };

        // Prevent burst of checks.
        if let Some(last_check_timestamp) = last_check_timestamp {
            if last_check_timestamp.elapsed().as_secs() < 5 {
                return;
            }
        }

        // Using read access to globals, detect if a local processes need to be re-started.
        //
        // Uses the services status stored in the localnet workdir globals.
        //
        // If the status is not "OK", and one of the service is shown as "NOT RUNNING" then
        // call "localnet start" to attempt recovery.
        let need_restart = {
            let globals_guard = self.globals.get_status(WORKDIR_IDX_LOCALNET).read().await;
            let globals = &*globals_guard;
            let mut need_restart = false;
            if let Some(ui) = &globals.ui {
                let ui = ui.get_data();
                if let Some(status) = &ui.status {
                    if status != "OK" {
                        if let Some(services) = &ui.services {
                            for service in services {
                                if let Some(service_status) = &service.status {
                                    if (service.label == "Localnet process"
                                        || service.label == "Faucet process")
                                        && service_status == "NOT RUNNING"
                                    {
                                        need_restart = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            need_restart
        };

        let mut update_recovery_timestamp = false;
        if need_restart {
            // Do nothing if last recovery attempt was less than 2 minutes ago.
            let mut send_restart_msg = true;

            if let Some(last_recovery_timestamp) = last_recovery_timestamp {
                if last_recovery_timestamp.elapsed().as_secs() < 120 {
                    send_restart_msg = false;
                }
            }

            // Restart the localnet service.
            if send_restart_msg {
                let mut msg = AdminControllerMsg::new();
                msg.event_id = EVENT_SHELL_EXEC;
                msg.workdir_idx = Some(WORKDIR_IDX_LOCALNET);
                msg.data_string = Some("localnet start --daemoncall".to_string());
                if let Err(e) = self.admctrl_tx.try_send(msg) {
                    log_safe!(format!(
                        "try_send EVENT_SHELL_EXEC localnet start failed: {}",
                        e
                    ));
                }
                update_recovery_timestamp = true;
            }
        }

        let workdir_tracking = self.wd_tracking.get_mut(WORKDIR_IDX_LOCALNET);
        let now = tokio::time::Instant::now();
        if update_recovery_timestamp {
            workdir_tracking.process_watchdog_last_recovery_timestamp = Some(now);
        }
        workdir_tracking.process_watchdog_last_check_timestamp = Some(now);
    }

    pub async fn send_event_post_publish(
        tx_channel: &AdminControllerTx,
        workdir_idx: WorkdirIdx,
    ) -> Result<()> {
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_POST_PUBLISH;
        msg.workdir_idx = Some(workdir_idx);
        if let Err(e) = tx_channel.try_send(msg) {
            let err_msg = format!(
                "try_send EVENT_POST_PUBLISH to admin controller failed: {}",
                e
            );
            log_safe!(err_msg);
            return Err(anyhow!(err_msg));
        }
        Ok(())
    }

    async fn process_post_publish_msg(&mut self, msg: AdminControllerMsg) {
        if msg.event_id != EVENT_POST_PUBLISH {
            log::error!(
                "process_post_publish_msg unexpected event_id {:?}",
                msg.event_id
            );
            return;
        }

        if msg.workdir_idx.is_none() {
            log::error!("process_post_publish_msg missing workdir_idx");
            return;
        }
        let workdir_idx = msg.workdir_idx.unwrap();

        // Just make the packages_poller do an update immediatly instead of
        // waiting until next periodical audit.
        if let Some(wd_tracking) = self.wd_tracking.get_if_some(workdir_idx) {
            let mut worker_msg = GenericChannelMsg::new();
            worker_msg.event_id = EVENT_UPDATE;
            worker_msg.workdir_idx = Some(workdir_idx);
            Self::send_msg_to_packages_poller(wd_tracking, worker_msg.clone()).await;
            // A CLI publish may have started the workdir services, so update that as well.
            Self::send_msg_to_cli_poller(wd_tracking, worker_msg).await;
        } else {
            log::error!(
                "process_post_publish_msg workdir tracking not found: {:?}",
                workdir_idx
            );
            return;
        }
    }

    async fn process_shell_exec_msg(&mut self, msg: AdminControllerMsg, subsys: &SubsystemHandle) {
        // Simply forward to the proper ShellWorker (one worker per workdir).
        if msg.event_id != EVENT_SHELL_EXEC {
            log::error!("Unexpected event_id {:?}", msg.event_id);
            // Do nothing. Consume the message.
            return;
        }

        if msg.workdir_idx.is_none() {
            log::error!("EVENT_SHELL_EXEC missing workdir_idx");
            return;
        }
        let workdir_idx = msg.workdir_idx.unwrap();

        // Find the corresponding ShellWorker in wd_tracking using the workdir_idx.
        let wd_tracking = self.wd_tracking.get_mut(workdir_idx);

        // Instantiate and start the ShellWorker if not already done.
        if wd_tracking.shell_worker_handle.is_none() {
            let (shell_worker_tx, shell_worker_rx) = tokio::sync::mpsc::channel(MPSC_Q_SIZE);
            wd_tracking.shell_worker_tx = Some(shell_worker_tx);
            let shell_worker = ShellWorker::new(shell_worker_rx, Some(workdir_idx));
            let nested = subsys.start(SubsystemBuilder::new("shell-worker", |a| {
                shell_worker.run(a)
            }));
            wd_tracking.shell_worker_handle = Some(nested);
        }

        if wd_tracking.shell_worker_tx.is_none() {
            log::error!("EVENT_SHELL_EXEC missing shell_worker_tx");
            return;
        }
        let shell_worker_tx = wd_tracking.shell_worker_tx.as_ref().unwrap();

        // Forward the message to the ShellWorker.
        let mut worker_msg = GenericChannelMsg::new();
        worker_msg.event_id = EVENT_EXEC;
        worker_msg.command = msg.data_string;
        worker_msg.workdir_idx = msg.workdir_idx;
        worker_msg.resp_channel = msg.resp_channel;
        if let Err(e) = shell_worker_tx.try_send(worker_msg) {
            let err_msg = format!("try_send EVENT_SHELL_EXEC to worker failed: {}", e);
            log_safe!(err_msg);
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

    fn apply_workdir_config(input_port: &mut InputPort, workdir_config: &WorkdirUserConfig) {
        let mut at_least_one_change = false;
        if input_port.is_proxy_enabled() != workdir_config.is_proxy_enabled() {
            input_port.set_proxy_enabled(workdir_config.is_proxy_enabled());
            at_least_one_change = true;
        }
        if input_port.is_user_request_start() != workdir_config.is_user_request_start() {
            input_port.set_user_request_start(workdir_config.is_user_request_start());
            at_least_one_change = true;
        }
        if input_port.target_servers.is_empty() {
            // Do a fast push of all. No need to check for TargetServer differences.
            for (_, config) in workdir_config.links().iter() {
                if config.rpc.is_some() {
                    input_port.add_target_server(config);
                }
            }
            if !input_port.target_servers.is_empty() {
                at_least_one_change = true;
            }
        } else {
            // Some TargetServer exists, so do a slower upsert.
            for (_, config) in workdir_config.links().iter() {
                if config.rpc.is_some() && input_port.upsert_target_server(config) {
                    at_least_one_change = true;
                }
            }
            // Handle excess TargetServers to remove.
            if input_port.target_servers.len() as usize > workdir_config.links().len() {
                // Iterate target_servers and take out the ones not in config.
                let mut to_remove: Vec<ManagedVecU8> = Vec::new();
                for (idx, target_server) in input_port.target_servers.iter_mut() {
                    let alias = target_server.alias();
                    if !workdir_config.links().contains_key(&alias) {
                        log::info!("Removing server {}", alias);
                        to_remove.push(idx);
                        at_least_one_change = true;
                    }
                }
                for idx in to_remove {
                    input_port.target_servers.remove(idx);
                }
            }
        }

        if at_least_one_change {
            input_port.update_selection_vectors();
        }
    }

    async fn process_config_msg(&mut self, msg: AdminControllerMsg, subsys: &SubsystemHandle) {
        // Detect any config change for one workdir, and apply it to all other runtime components.

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

        // Load the configuration.
        let mut workdir_config = WorkdirUserConfig::new();
        let workdir_idx: u8;
        let workdir_name: String;
        {
            let workdirs_guard = self.globals.workdirs.read().await;
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

            // Load the 3 suibase.yaml files. The default, common and user version in order.
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
        let config_applied: Option<(ManagedVecU8, u16)> = {
            // Get a write lock on the globals.
            let mut globals_guard = self.globals.proxy.write().await;
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

            // Create the InputPort if does not exists.
            if let Some((port_idx, input_port)) = input_port_search {
                // Modifying an existing InputPort.
                Self::apply_workdir_config(input_port, &workdir_config);
                Some((port_idx, input_port.port_number()))
            } else {
                // TODO Verify there is no conflicting port assignment.

                // No InputPort yet for that workdir... so create it.
                let mut input_port =
                    InputPort::new(workdir_idx, workdir_name.clone(), &workdir_config);
                Self::apply_workdir_config(&mut input_port, &workdir_config);
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
                let globals = self.globals.proxy.clone();
                let netmon_tx = self.netmon_tx.clone();
                let nested = subsys.start(SubsystemBuilder::new("proxy-server", move |a| {
                    proxy_server.run(a, port_idx, globals, netmon_tx)
                }));

                port_tracking.proxy_server_handle = Some(nested);
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
                common::mpsc_q_check!(self.admctrl_rx);
                match msg.event_id {
                    EVENT_AUDIT => {
                        self.process_audit_msg(msg).await;
                    }
                    EVENT_DEBUG_PRINT => {
                        self.process_debug_print_msg(msg).await;
                    }
                    EVENT_NOTIF_CONFIG_FILE_CHANGE => {
                        self.process_config_msg(msg, subsys).await;
                    }
                    EVENT_SHELL_EXEC => {
                        self.process_shell_exec_msg(msg, subsys).await;
                    }
                    EVENT_UPDATE => {
                        self.process_update_msg(msg).await;
                    }
                    EVENT_POST_PUBLISH => {
                        self.process_post_publish_msg(msg).await;
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
            let workdirs_watcher = WorkdirsWatcher::new(self.globals.workdirs.clone(), admctrl_tx);
            subsys.start(SubsystemBuilder::new("workdirs-watcher", move |a| {
                workdirs_watcher.run(a)
            }));
        }

        // Create a WorkdirTracking for every possible workdir.
        for workdir_idx in 0..WORKDIRS_KEYS.len() {
            let workdir_idx = workdir_idx as u8;
            let wd_tracking = self.wd_tracking.get_mut(workdir_idx);

            // Starts the task handling Sui events for latest published packages.
            if workdir_idx == WORKDIR_IDX_LOCALNET {
                if wd_tracking.events_worker_handle.is_none() {
                    let (events_worker_tx, events_worker_rx) =
                        tokio::sync::mpsc::channel(MPSC_Q_SIZE);

                    let events_worker_params = EventsWriterWorkerParams::new(
                        self.globals.clone(),
                        events_worker_rx,
                        events_worker_tx.clone(),
                        workdir_idx,
                    );
                    wd_tracking.events_worker_tx = Some(events_worker_tx);

                    let events_worker = EventsWriterWorker::new(events_worker_params);
                    let nested = subsys.start(SubsystemBuilder::new(
                        format!("events-worker-{}", workdir_idx),
                        |a| events_worker.run(a),
                    ));
                    wd_tracking.events_worker_handle = Some(nested);
                }
            }

            // Start a CLI poller.
            if wd_tracking.cli_poller.is_none() {
                let params = CliPollerParams::new(
                    self.globals.clone(),
                    self.admctrl_tx.clone(),
                    workdir_idx,
                );

                let poller = CliPoller::new(params, &subsys);

                wd_tracking.cli_poller = Some(poller);
            }

            // Start a packages poller.
            if wd_tracking.packages_poller.is_none() {
                let params = PackagesPollerParams::new(
                    self.globals.clone(),
                    wd_tracking.events_worker_tx.clone(),
                    workdir_idx,
                );

                let poller = PackagesPoller::new(params, &subsys);
                wd_tracking.packages_poller = Some(poller);
            }
        }

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

impl ManagedElement for AdminController {
    fn idx(&self) -> Option<ManagedVecU8> {
        self.idx
    }
    fn set_idx(&mut self, idx: Option<ManagedVecU8>) {
        self.idx = idx;
    }
}

#[cfg(test)]
use crate::shared_types::GlobalsWorkdirsST;

#[test]
fn test_load_config_from_suibase_default() {
    // Note: More of a functional test. Suibase need to be installed.

    // Test a known "standard" localnet suibase.yaml
    let workdirs = GlobalsWorkdirsST::new();
    let mut path = std::path::PathBuf::from(workdirs.suibase_home());
    path.push("scripts");
    path.push("defaults");
    path.push("localnet");
    path.push("suibase.yaml");

    let workdir_search_result = workdirs.find_workdir(&path.to_string_lossy().to_string());
    assert!(workdir_search_result.is_some());
    let (_workdir_idx, workdir) = workdir_search_result.unwrap();

    let mut config = WorkdirUserConfig::new();
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
    //   rpc: "http://localhost:9000"
    //   ws: "ws://localhost:9000"
    assert_eq!(config.links_overrides(), false);
    assert_eq!(config.links().len(), 1);
    assert!(config.links().contains_key("localnet"));
    assert!(config.links().get("localnet").unwrap().rpc.is_some());
    assert!(config.links().get("localnet").unwrap().ws.is_some());
    let link = config.links().get("localnet").unwrap();
    assert_eq!(link.rpc.as_ref().unwrap(), "http://localhost:9000");
    assert_eq!(link.ws.as_ref().unwrap(), "ws://localhost:9000");
}
