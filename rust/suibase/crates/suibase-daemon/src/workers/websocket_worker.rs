// Child thread of events_writer_worker
//
// Responsible to:
//   - websocket auto-reconnect for a single server.
//   - keep alive the connection with Ping
//   - subscribe/unsubscribe to Sui events, filter and forward the
//     validated data to its parent thread.
//
// The thread is auto-restart in case of panic.

use std::{collections::HashMap, sync::Arc};

use crate::{
    basic_types::{
        self, AutoThread, GenericChannelMsg, GenericRx, GenericTx, Runnable, WorkdirIdx,
    },
    shared_types::{Globals, GlobalsPackagesConfigST},
};

use anyhow::Result;
use axum::async_trait;

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};

use super::package_tracking::{PackageTracking, PackageTrackingState};

#[derive(Clone)]
pub struct WebSocketWorkerParams {
    globals: Globals,
    event_rx: Arc<Mutex<GenericRx>>,
    event_tx: GenericTx,
    workdir_idx: WorkdirIdx,
}

impl WebSocketWorkerParams {
    pub fn new(
        globals: Globals,
        event_rx: GenericRx,
        event_tx: GenericTx,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            event_tx,
            workdir_idx,
        }
    }
}

pub struct WebSocketWorker {
    auto_thread: AutoThread<WebSocketWorkerThread, WebSocketWorkerParams>,
}

impl WebSocketWorker {
    pub fn new(params: WebSocketWorkerParams) -> Self {
        Self {
            auto_thread: AutoThread::new("WebSocketWorker".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

#[derive(Debug, Default)]
struct WebSocketManagement {
    // Active websocket connection.
    write: Option<SplitSink<WebSocketStream<TcpStream>, Message>>,
    read: Option<SplitStream<WebSocketStream<TcpStream>>>,

    // Sequence number to use as "id" for JSON-RPC.
    // Must be incremented prior to use it in a new request.
    seq_number: u64,
}

impl WebSocketManagement {
    pub fn new() -> Self {
        // TODO Initialize sequence number with a UTC in milliseconds.
        Self {
            write: None,
            read: None,
            seq_number: 0,
        }
    }
}

struct WebSocketWorkerThread {
    thread_name: String,
    params: WebSocketWorkerParams,

    // Key is the package_id.
    packages: HashMap<String, PackageTracking>,

    websocket: WebSocketManagement,
}

#[async_trait]
impl Runnable<WebSocketWorkerParams> for WebSocketWorkerThread {
    fn new(thread_name: String, params: WebSocketWorkerParams) -> Self {
        Self {
            thread_name,
            params,
            packages: HashMap::new(),
            websocket: WebSocketManagement::new(),
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

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

impl WebSocketWorkerThread {
    fn subscribe_request_format(id: u64, package_id: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","method":"suix_subscribeEvent","id":{},"params":[{{"Package":"{}"}}]}}"#,
            id, package_id
        )
    }

    fn unsubscribe_request_format(id: u64, unsubscribe_id: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","method":"suix_unsubscribeEvent","id":{},"params":[{}]}}"#,
            id, unsubscribe_id
        )
    }

    async fn process_ws_msg(&mut self, msg: Message) {
        //log::info!("Received a websocket message: {:?}", msg);

        let (json_msg, msg_seq_number) = match msg {
            Message::Text(text) => {
                let json = serde_json::from_str(&text);
                if json.is_err() {
                    log::error!("Failed to parse JSON: {:?}", text);
                    return;
                }
                let json_msg: serde_json::Value = json.unwrap();
                let id = json_msg["id"].as_u64().unwrap_or(0);
                (json_msg, id)
            }
            _ => {
                log::error!("Unexpected websocket message: {:?}", msg);
                return;
            }
        };

        // Check for expected response (correlate using the JSON-RPC id).
        let mut trig_audit_event = false;
        let mut correlated_msg = false;
        for package in self.packages.values_mut() {
            let state = package.state();
            if state == &PackageTrackingState::Subscribing {
                if package.did_sent_subscribe_request(msg_seq_number) {
                    correlated_msg = true;
                    log::info!(
                        "Received websocket subscribe resp: {:?} for package id {}",
                        json_msg,
                        package.id()
                    );
                    // Got an expected subscribe response.
                    // Extract the result string from the JSON message.
                    let result = json_msg["result"].as_u64();
                    if result.is_none() {
                        log::error!("Missing result in subscribe JSON response: {:?}", json_msg);
                        return;
                    }
                    let unsubscribe_id = result.unwrap();
                    package.report_subscribing_response(unsubscribe_id.to_string());
                    trig_audit_event = true;
                    break;
                }
            } else if state == &PackageTrackingState::Unsubscribing
                && package.did_sent_unsubscribe_request(msg_seq_number)
            {
                // Got an expected unsubscribe response.
                correlated_msg = true;
                log::info!(
                    "Received websocket unsubscribe resp: {:?} for package id {}",
                    json_msg,
                    package.id()
                );

                package.report_unsubscribing_response();
                trig_audit_event = true;
                break;
            }
        }
        if !correlated_msg {
            log::error!("Received websocket message: {:?}", json_msg);
        }

        if trig_audit_event {
            let msg = GenericChannelMsg {
                event_id: basic_types::EVENT_AUDIT,
                data_string: None,
                workdir_idx: Some(self.params.workdir_idx),
                resp_channel: None,
            };
            if self.params.event_tx.send(msg).await.is_err() {
                log::error!(
                    "Failed to send audit message for workdir_idx={}",
                    self.params.workdir_idx
                );
            }
        }
    }

    async fn process_audit_msg(&mut self, msg: GenericChannelMsg) {
        // This function takes care of operation that need to sync
        // between self.packages and the packages_config information.
        //
        // Changes to packages_config are NOT allowed here. See process_update_msg()
        // for operations that requires touching the packages_config globals.

        if msg.event_id != basic_types::EVENT_AUDIT {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
                return;
            }
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
            return;
        }

        // log::info!("Received an audit message: {:?}", msg);
        let mut state_change = false;
        {
            // Get a reader lock on the globals packages_config.
            let globals_read_guard = self.params.globals.packages_config.read().await;
            let workdirs = &globals_read_guard.workdirs;

            // Get the element in packages_config for workdir_idx.

            let move_configs =
                GlobalsPackagesConfigST::get_move_configs(workdirs, self.params.workdir_idx);
            if move_configs.is_none() {
                return; // Normal when the workdir never had any published package.
            }
            let move_configs = move_configs.unwrap();

            // Check for adding PackagesTracking.
            // Add a PackagesTracking in the packages HashMap for every latests in packages_config.
            // Once created, the PackagesTracking remains until removed from packages_config.
            // The package_id is used as the key in the packages HashMap.
            for (uuid, move_config) in move_configs {
                let latest = move_config.latest_package.as_ref().unwrap();
                // Check if the package is already in the packages HashMap.
                if !self.packages.contains_key(&latest.package_id) {
                    if move_config.path.is_none() {
                        log::error!("Missing path in move_config {:?}", move_config);
                        continue;
                    }
                    let toml_path = move_config.path.as_ref().unwrap().clone();

                    // Create a new PackagesTracking.
                    let package_tracking = PackageTracking::new(
                        toml_path,
                        latest.package_name.clone(),
                        uuid.to_string(),
                        latest.package_id.clone(),
                    );
                    // Add the PackagesTracking to the packages HashMap.
                    self.packages
                        .insert(latest.package_id.clone(), package_tracking);
                }
            }

            // Transition package to Unsubscribing state when no longer in the config.
            // Remove the package tracking once unsubscription confirmed (or timeout).
            self.packages.retain(|package_id, package_tracking| {
                let mut retain = true;
                let move_config = move_configs.get(package_tracking.uuid().as_str());
                if let Some(move_config) = move_config {
                    // Verify if this package_id is still the latest published for this package UUID.
                    if move_config.latest_package.is_none() {
                        retain = false;
                    } else {
                        let latest = move_config.latest_package.as_ref().unwrap();
                        if latest.package_id != *package_id {
                            retain = false;
                        }
                    }
                } else {
                    retain = false;
                }
                if !retain {
                    if package_tracking.can_be_deleted() {
                        log::info!("Deleting tracking for package_id={}", package_id);
                        return false; // Delete the element in the HashMap.
                    }
                    // Transition toward eventual deletion after Unsubscribing completes (or timeout).
                    if !package_tracking.is_remove_requested() {
                        package_tracking.report_remove_request();
                    }
                }
                true // Keep the element in the HashMap.
            });
        } // End of reader lock.

        let websocket = &mut self.websocket;
        let packages = &mut self.packages;

        // TODO Transition here to Disconnected or ReadyToDelete on connection lost?

        // Check to update every PackagesTracking state machine.
        for package in packages.values_mut() {
            if package.is_remove_requested() {
                //log::info!("Initiating processing removed from package");
                if Self::try_to_unsubscribe(package, websocket).await {
                    state_change = true;
                }
            } else {
                match package.state() {
                    PackageTrackingState::Disconnected => {
                        // Initial state.
                        if Self::try_to_subscribe(package, websocket).await {
                            state_change = true;
                        }
                    }
                    PackageTrackingState::Subscribing => {
                        if Self::try_to_subscribe(package, websocket).await {
                            state_change = true;
                        }
                    }
                    PackageTrackingState::Subscribed => {
                        // Nothing to do.
                        // Valid next states are Unsubscribing (removed from config) or Disconnected (on connection loss).
                    }
                    PackageTrackingState::Unsubscribing => {
                        // Valid next state is Unsubscribed (on unsubscribed confirmation, timeout) and ReadyToDelete (on connection loss).
                        if Self::try_to_unsubscribe(package, websocket).await {
                            state_change = true;
                        }
                    }
                    PackageTrackingState::ReadyToDelete => {
                        // End state. Nothing to do. The package will eventually be deleted on next audit.
                    }
                }
            }
        }

        if state_change {
            // Update the packages_config globals.
            let msg = GenericChannelMsg {
                event_id: basic_types::EVENT_UPDATE,
                data_string: None,
                workdir_idx: Some(self.params.workdir_idx),
                resp_channel: None,
            };
            if self.params.event_tx.send(msg).await.is_err() {
                log::error!(
                    "Failed to send update message for workdir_idx={}",
                    self.params.workdir_idx
                );
            }
        }
    }

    async fn process_update_msg(&mut self, msg: GenericChannelMsg) {
        // This function takes care of synching from self.packages to
        // the global packages_config.
        //
        // Unlike an audit, changes to packages_config globals are
        // allowed here.
        log::info!("Received an update message: {:?}", msg);

        // Make sure the event_id is EVENT_UPDATE.
        if msg.event_id != basic_types::EVENT_UPDATE {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
                return;
            }
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
            return;
        }

        let mut trig_audit = false;
        {
            // Get a writer lock on the globals packages_config.
            let mut globals_write_guard = self.params.globals.packages_config.write().await;
            let globals = &mut *globals_write_guard;

            // Get the element in packages_config for workdir_idx.

            let move_configs = GlobalsPackagesConfigST::get_mut_move_configs(
                &mut globals.workdirs,
                self.params.workdir_idx,
            );

            // Check for adding PackagesTracking.
            // Add a PackagesTracking in the packages HashMap for every latests in packages_config.
            // Once created, the PackagesTracking remains until removed from packages_config.
            // The package_id is used as the key in the packages HashMap.
            for (uuid, move_config) in &mut *move_configs {
                let latest = move_config.latest_package.as_ref().unwrap();
                // Check if the package is already in the packages HashMap.
                if !self.packages.contains_key(&latest.package_id) {
                    if move_config.path.is_none() {
                        log::error!("Missing path in move_config {:?}", move_config);
                        continue;
                    }
                    let toml_path = move_config.path.as_ref().unwrap().clone();

                    // Create a new PackagesTracking.
                    let package_tracking = PackageTracking::new(
                        toml_path,
                        latest.package_name.clone(),
                        uuid.to_string(),
                        latest.package_id.clone(),
                    );
                    // Add the PackagesTracking to the packages HashMap.
                    self.packages
                        .insert(latest.package_id.clone(), package_tracking);
                    trig_audit = true;
                } else {
                    let package_tracking = &self.packages[&latest.package_id];
                    let package_tracking_state: u32 = package_tracking.state().clone().into();
                    if move_config.tracking_state != package_tracking_state {
                        move_config.tracking_state = package_tracking_state;
                    }
                }
            }
        }

        if trig_audit {
            let msg = GenericChannelMsg {
                event_id: basic_types::EVENT_AUDIT,
                data_string: None,
                workdir_idx: Some(self.params.workdir_idx),
                resp_channel: None,
            };
            if self.params.event_tx.send(msg).await.is_err() {
                log::error!(
                    "Failed to send audit message for workdir_idx={}",
                    self.params.workdir_idx
                );
            }
        }
    }

    async fn try_to_subscribe(
        package: &mut PackageTracking,
        websocket: &mut WebSocketManagement,
    ) -> bool {
        // Send a subscribe message, unless there is one already recently pending.
        // On failure, keep retrying as long that package is configured.
        // (retry will be on subsequent call).
        //
        // Return true if there is a state change.
        let mut state_change = false;
        match package.state() {
            PackageTrackingState::Disconnected => {
                // Valid state when calling this function.
                if package.change_state_to(PackageTrackingState::Subscribing) {
                    state_change = true;
                }
            }
            PackageTrackingState::Subscribing => {
                if package.unsubscribed_id().is_some() {
                    if package.change_state_to(PackageTrackingState::Subscribed) {
                        state_change = true;
                    }
                    return state_change;
                }
            }
            _ => {
                // All set. Nothing to do.
                return false;
            }
        };

        let mut send_subscribe_message = true;

        // Don't do it if one was already sent in last 2 seconds.
        if package.secs_since_last_request() < 2 {
            send_subscribe_message = false;
        }

        if send_subscribe_message {
            // Check if retrying and log error only on first retry and once in a while after.
            if package.request_retry() % 3 == 1 {
                log::error!("Failed to subscribe package_id={}", package.id());
            }
            websocket.seq_number += 1;
            package.report_subscribing_request(websocket.seq_number);
            let msg = Message::Text(Self::subscribe_request_format(
                websocket.seq_number,
                &package.id().clone(), // Must not have leading 0x
            ));

            if let Some(ref mut write) = websocket.write {
                log::info!("Sending subscribe message: {:?}", msg);
                if let Err(e) = write.send(msg).await {
                    log::error!("subscribe write.send error: {:?}", e);
                } else {
                    log::info!("subscribe write.send success");
                }
            }
        }

        state_change
    }

    async fn try_to_unsubscribe(
        package: &mut PackageTracking,
        websocket: &mut WebSocketManagement,
    ) -> bool {
        // If subscribed, then send a unsubscribe message, unless there is one
        // already recently pending.
        //
        // On failure, keep retrying until timeout (retry will be on subsequent call).
        // After being confirmed unsubscribe (or timeout) the PackageTracking state
        // becomes ReadyToDelete.
        let mut state_change = false;
        match package.state() {
            PackageTrackingState::Disconnected => {
                // No subscription on-going...
                if package.change_state_to(PackageTrackingState::ReadyToDelete) {
                    state_change = true;
                }
                return state_change;
            }
            PackageTrackingState::Subscribing => {
                // If trying to unsubscribe while a subscription request was already sent (and
                // no response receive yet), then let the subscription a chance to complete.
                // This will allow for a clean unsubscribe later.
                // Check for a subscription timeout transition to avoid being block forever.
                if package.is_subscribe_request_pending_response()
                    && package.secs_since_last_request() >= 2
                {
                    // Do nothing... to give a chance for the subscription to succeed.
                    state_change = false;
                    return state_change;
                }

                if package.change_state_to(PackageTrackingState::Unsubscribing) {
                    state_change = true;
                }
                return state_change;
            }
            PackageTrackingState::Subscribed => {
                if package.change_state_to(PackageTrackingState::Unsubscribing) {
                    state_change = true;
                }
                return state_change;
            }

            PackageTrackingState::Unsubscribing => {
                // Ready to delete if unsubscribed_id is clear or timeout.
                // The unsubscribed_id is clear when receiving a unsubscribe response.
                if package.unsubscribed_id().is_none() || package.request_retry() > 10 {
                    if package.change_state_to(PackageTrackingState::ReadyToDelete) {
                        state_change = true;
                    }
                    return state_change;
                }
            }

            PackageTrackingState::ReadyToDelete => {
                // Nothing to do.
                state_change = false;
                return state_change;
            }
        };

        // If there is no known unsubscribed_id, then no point to try to unsubscribe.
        if package.unsubscribed_id().is_none() {
            if package.change_state_to(PackageTrackingState::ReadyToDelete) {
                state_change = true;
            }
            return state_change;
        }

        let mut send_unsubscribe_message = true;
        // Don't do it if one was already sent in last 2 seconds.
        if package.secs_since_last_request() < 2 {
            send_unsubscribe_message = false;
        }

        if send_unsubscribe_message {
            // Periodically report an error on too many retry.
            if package.request_retry() % 3 == 1 {
                log::error!("Failed to unsubscribe package_id={}", package.id());
            }
            websocket.seq_number += 1;
            package.report_unsubscribing_request(websocket.seq_number);
            let msg = Message::Text(Self::unsubscribe_request_format(
                websocket.seq_number,
                package.unsubscribed_id().unwrap(), // Must not have leading 0x
            ));

            if let Some(ref mut write) = websocket.write {
                log::info!("Sending unsubscribe message: {:?}", msg);
                if let Err(e) = write.send(msg).await {
                    log::error!("unsubscribe write.send error: {:?}", e);
                } else {
                    log::info!("unsubscribe write.send success");
                }
            }
        }

        state_change
    }

    async fn open_websocket(&mut self) -> bool {
        // Open a websocket connection to the server for this workdir.
        // TODO Change this to the actual server URL from the config.
        let socket_url = "ws://0.0.0.0:9000";

        match connect_async(socket_url).await {
            Ok((ws_stream, _response)) => {
                let (write, read) = ws_stream.split();
                self.websocket.write = Some(write);
                self.websocket.read = Some(read);
            }
            Err(e) => {
                log::error!("connect_async error: {:?}", e);
                self.websocket.write = None;
                self.websocket.read = None;
            }
        }

        self.websocket.write.is_some()
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // Take mutable ownership of the event_rx channel as long this thread is running.
        let event_rx = Arc::clone(&self.params.event_rx);
        let mut event_rx = event_rx.lock().await;

        // Check to establish a websocket connection (as needed).
        if self.websocket.write.is_none() && !self.open_websocket().await {
            // Delay of 5 seconds before retrying.
            // TODO Replace delay with checking time elapsed.
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            return;
        }

        while !subsys.is_shutdown_requested() {
            let ws_stream_future =
                futures::FutureExt::fuse(self.websocket.read.as_mut().unwrap().next());
            let event_rx_future = futures::FutureExt::fuse(event_rx.recv());

            tokio::select! {
                msg = ws_stream_future => {
                    if let Some(msg) = msg {
                        let msg = msg.unwrap();
                        self.process_ws_msg(msg).await;
                    } else {
                        // Shutdown requested.
                        log::info!("Received a None websocket message");
                        return;
                    }
                }
                msg = event_rx_future => {
                    if let Some(msg) = msg {
                        // Process the message.
                        match msg.event_id {
                            basic_types::EVENT_AUDIT => {
                                self.process_audit_msg(msg).await;
                            },
                            basic_types::EVENT_UPDATE => {
                                self.process_update_msg(msg).await;
                            },
                            _ => {
                                // Consume unexpected messages.
                                log::error!("Unexpected event_id {:?}", msg );
                            }
                        }
                    } else {
                        // Channel closed or shutdown requested.
                        log::info!("Received a None internal message");
                        return;
                    }
                }
            }
        }
    }
}
