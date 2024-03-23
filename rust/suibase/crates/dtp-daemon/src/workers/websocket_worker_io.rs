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

use crate::shared_types::{
    ExtendedWebSocketWorkerIOMsg, Globals, GlobalsPackagesConfigST, WebSocketWorkerIOMsg,
    WebSocketWorkerIORx, WebSocketWorkerIOTx, WebSocketWorkerMsg, WebSocketWorkerTx,
};

use common::shared_types::{
    WORKDIRS_KEYS, WORKDIR_IDX_DEVNET, WORKDIR_IDX_LOCALNET, WORKDIR_IDX_MAINNET,
    WORKDIR_IDX_TESTNET,
};

use common::basic_types::{self, AutoThread, GenericChannelMsg, Runnable, WorkdirIdx};

use anyhow::{bail, Result};
use axum::async_trait;

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::info;
use serde_json::{Map, Value};
use sui_types::base_types::ObjectID;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use common::workers::{SubscriptionTracking, SubscriptionTrackingState};

#[derive(Clone)]
pub struct WebSocketWorkerIOParams {
    globals: Globals,
    event_rx: Arc<Mutex<WebSocketWorkerIORx>>, // Input message queue to this worker.
    self_tx: WebSocketWorkerIOTx,              // To send message to self.
    parent_tx: WebSocketWorkerTx,              // To send message to parent
    workdir_idx: WorkdirIdx,
    workdir_name: String,
}

impl WebSocketWorkerIOParams {
    pub fn new(
        globals: Globals,
        event_rx: WebSocketWorkerIORx,
        event_tx: WebSocketWorkerIOTx,
        parent_tx: WebSocketWorkerTx,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            self_tx: event_tx,
            parent_tx,
            workdir_idx,
            workdir_name: WORKDIRS_KEYS[workdir_idx as usize].to_string(),
        }
    }
}

pub struct WebSocketWorkerIO {
    auto_thread: AutoThread<WebSocketWorkerIOThread, WebSocketWorkerIOParams>,
}

impl WebSocketWorkerIO {
    pub fn new(params: WebSocketWorkerIOParams) -> Self {
        Self {
            auto_thread: AutoThread::new("WebSocketWorker".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

#[derive(Debug, Default)]
struct WebSocketIOManagement {
    // Active websocket connection.
    write: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    read: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,

    // Sequence number to use as "id" for JSON-RPC.
    // Must be incremented prior to use it in a new request.
    seq_number: u64,
}

impl WebSocketIOManagement {
    pub fn new() -> Self {
        // TODO Initialize sequence number with a UTC in milliseconds.
        Self {
            write: None,
            read: None,
            seq_number: 0,
        }
    }
}

#[derive(Debug, Default)]
struct InnerPipeTracking {
    subs: SubscriptionTracking,
}

type InnerPipeTrackingMap = HashMap<String, InnerPipeTracking>;

#[derive(Debug, Default)]
struct ClientConnTracking {
    // For convenience. Set once on instantiation.
    // host_sla_idx: u16,

    // To track events from localhost InnerPipes.
    // The key is an InnerPipe address ("0x" string).
    ipipe_trackings: InnerPipeTrackingMap,
}
type ClientConnTrackingMap = HashMap<String, ClientConnTracking>;

#[derive(Debug, Default)]
struct ServerConnTracking {
    // For convenience. Set once on instantiation.
    //host_sla_idx: u16,

    // To track events from InnerPipes for an incoming connection.
    // The key is an InnerPipe address ("0x" string).
    ipipe_trackings: InnerPipeTrackingMap,
}
type ServerConnTrackingMap = HashMap<String, ServerConnTracking>;

struct WebSocketWorkerIOThread {
    thread_name: String,
    params: WebSocketWorkerIOParams,

    // Key is the object address ("0x" string).
    package_subs: HashMap<String, SubscriptionTracking>,

    // To track events from localhost objects (e.g. ConnReq).
    localhost_subs: HashMap<ObjectID, SubscriptionTracking>,

    // Subscribe to to all events coming from the DTP Move module package
    // with additional filter on the sender address and the receiver field.
    //
    // Key is the (Package, Sender) address tuple ("0x" strings)
    //sender_subs: HashMap<(String, String), SubscriptionTracking>,

    // TransportController/Pipes/InnerPipes Tracking
    //
    // Key is a TransportController Sui address ("0x" string).
    //
    // This is only for the objects to be used
    // for *incoming* traffic/events.
    //
    // In other word, only for the pipes/ipipes *owned*
    // by the localhost(s).
    //
    // The dtp-daemon can be both client and server (on different
    // connections), so there is two maps.
    //
    cli_conns: ClientConnTrackingMap,
    srv_conns: ServerConnTrackingMap,

    websocket: WebSocketIOManagement,
}

#[async_trait]
impl Runnable<WebSocketWorkerIOParams> for WebSocketWorkerIOThread {
    fn new(thread_name: String, params: WebSocketWorkerIOParams) -> Self {
        Self {
            thread_name,
            params,
            package_subs: HashMap::new(),
            localhost_subs: HashMap::new(),
            cli_conns: HashMap::new(),
            srv_conns: HashMap::new(),
            websocket: WebSocketIOManagement::new(),
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        // let output = format!("started {}", self.params.workdir_name);
        // log::info!("{}", output);

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                // log::info!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

impl WebSocketWorkerIOThread {
    fn subscribe_request_format(
        json_id: u64,
        package_id: &str,
        src_addr: Option<&String>,
        sender: Option<&String>,
    ) -> String {
        let mut filter = format!(r#"{{"Package":"{}"}}"#, package_id);

        if let Some(sender) = sender {
            let append_filter = format!(r#"{{"Sender":"{}"}}"#, sender);
            filter = format!(r#"{{"And": [{},{}]}}"#, filter, append_filter);
        }

        if let Some(src_addr) = src_addr {
            let append_filter = format!(
                r#"{{"MoveEventField":{{"path":"/src_addr", "value":"{}"}}}}"#,
                src_addr
            );
            filter = format!(r#"{{"And": [{},{}]}}"#, filter, append_filter);
        }

        let req = format!(
            r#"{{"jsonrpc":"2.0","method":"suix_subscribeEvent","id":{},"params":[{}]}}"#,
            json_id, filter
        );
        req
    }

    fn unsubscribe_request_format(json_id: u64, unsubscribe_id: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","method":"suix_unsubscribeEvent","id":{},"params":[{}]}}"#,
            json_id, unsubscribe_id
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
            Message::Ping(_) => {
                if let Some(ref mut write) = self.websocket.write {
                    // Pong are automatically queued by tungstenite, just need to flush them once in while.
                    // https://docs.rs/tungstenite/latest/tungstenite/protocol/struct.WebSocket.html#method.flush
                    if let Err(e) = write.flush().await {
                        log::error!("flush write.send error: {:?}", e);
                    }
                }
                return;
            }
            _ => {
                log::error!("Unexpected websocket message: {:?}", msg);
                return;
            }
        };

        // Check for expected response (correlate using the JSON-RPC id).
        let mut trig_audit_event = false;
        let mut is_correlated_msg = false;
        if msg_seq_number != 0 {
            for tracker in self.package_subs.values_mut() {
                let (a, b) = Self::tracker_update_state_correlation(
                    tracker,
                    &json_msg,
                    msg_seq_number,
                    &self.params.workdir_name,
                );
                if a {
                    is_correlated_msg = true;
                }
                if b {
                    trig_audit_event = true;
                }
            }

            if !is_correlated_msg {
                for tracker in self.localhost_subs.values_mut() {
                    let (a, b) = Self::tracker_update_state_correlation(
                        tracker,
                        &json_msg,
                        msg_seq_number,
                        &self.params.workdir_name,
                    );
                    if a {
                        is_correlated_msg = true;
                    }
                    if b {
                        trig_audit_event = true;
                    }
                }
            }

            if !is_correlated_msg {
                for tracker in self.cli_conns.values_mut() {
                    for ipipe in tracker.ipipe_trackings.values_mut() {
                        let (a, b) = Self::tracker_update_state_correlation(
                            &mut ipipe.subs,
                            &json_msg,
                            msg_seq_number,
                            &self.params.workdir_name,
                        );
                        if a {
                            is_correlated_msg = true;
                        }
                        if b {
                            trig_audit_event = true;
                        }
                    }
                }
            }

            if !is_correlated_msg {
                // Check with ipipes subscriptions.
                for tracker in self.srv_conns.values_mut() {
                    for ipipe in tracker.ipipe_trackings.values_mut() {
                        let (a, b) = Self::tracker_update_state_correlation(
                            &mut ipipe.subs,
                            &json_msg,
                            msg_seq_number,
                            &self.params.workdir_name,
                        );
                        if a {
                            is_correlated_msg = true;
                        }
                        if b {
                            trig_audit_event = true;
                        }
                    }
                }
            }
        }

        if trig_audit_event {
            let generic_msg = GenericChannelMsg {
                event_id: basic_types::EVENT_AUDIT,
                command: None,
                params: Vec::new(),
                data_json: None,
                workdir_idx: Some(self.params.workdir_idx),
                resp_channel: None,
            };
            let ws_io_msg = WebSocketWorkerIOMsg::Generic(generic_msg);
            if self.params.self_tx.send(ws_io_msg).await.is_err() {
                log::error!(
                    "Failed to send audit message for workdir_idx={}",
                    self.params.workdir_idx
                );
            }
        }

        if !is_correlated_msg {
            info!("Processing uncorrelated message: {:?}", json_msg);
            // Check if a valid Sui event message.
            let method = json_msg.get("method");
            if method.is_none() {
                log::error!(
                    "Missing method field in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let method = method.unwrap().as_str().unwrap_or("");

            if method != "suix_subscribeEvent" {
                log::error!(
                    "Unexpected method in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }

            let params = json_msg.get("params");
            if params.is_none() {
                log::error!(
                    "Missing params in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let params = params.unwrap().as_object();
            if params.is_none() {
                log::error!(
                    "Invalid params object in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let params = params.unwrap();

            let subscription = params.get("subscription");
            if subscription.is_none() {
                log::error!(
                    "Missing subscription in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let subscription_number = subscription.unwrap().as_u64();
            if subscription_number.is_none() {
                log::error!(
                    "Invalid subscription in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let subscription_number = subscription_number.unwrap();

            let result = params.get("result");
            if result.is_none() {
                log::error!(
                    "Missing result in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let result = result.unwrap().as_object();
            if result.is_none() {
                log::error!(
                    "Invalid result object in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let result = result.unwrap();

            let parsed_json = result.get("parsedJson");
            if parsed_json.is_none() {
                log::error!(
                    "Missing parsed_json in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let parsed_json = parsed_json.unwrap().as_object();
            if parsed_json.is_none() {
                log::error!(
                    "Invalid parsed_json object in Sui Event message. workdir={} message={:?}",
                    self.params.workdir_name,
                    json_msg
                );
                return;
            }
            let parsed_json = parsed_json.unwrap();

            // Optional src field
            //
            // Indicates the origin of the event for a first coarse classification:
            //   1 : DTP client tx ipipe.
            //   2 : DTP server tx ipipe.
            //   3 : DTP Host object
            //   4 : Suibase Console Log event.
            //
            // Assume this is a DTP message when both src and src_addr are defined,
            let mut src_candidate: Option<u64> = None;
            if let Some(x) = parsed_json.get("src") {
                src_candidate = x.as_u64();
            }

            let mut src_addr_candidate: Option<&str> = None;
            if let Some(x) = parsed_json.get("src_addr") {
                src_addr_candidate = x.as_str();
            }

            // If one is missing, assume this is not a DTP message.
            let is_dtp_message = src_candidate.is_some() && src_addr_candidate.is_some();
            let dtp_src = if is_dtp_message {
                src_candidate.unwrap()
            } else {
                0
            };
            let dtp_src_addr = if is_dtp_message {
                src_addr_candidate.unwrap()
            } else {
                ""
            };

            // Extract TransportController field (when applicable).
            let tc_addr = if dtp_src == 1 || dtp_src == 2 {
                let tc_ref = parsed_json.get("tc_ref");
                if tc_ref.is_none() {
                    log::error!(
                        "Missing tc_ref in DTP Sui Event message. workdir={} message={:?}",
                        self.params.workdir_name,
                        json_msg
                    );
                    return;
                }
                let tc_ref = tc_ref.unwrap().as_object();
                if tc_ref.is_none() {
                    log::error!(
                        "Invalid tc_ref in DTP Sui Event message. workdir={} message={:?}",
                        self.params.workdir_name,
                        json_msg
                    );
                    return;
                }
                let tc_ref = tc_ref.unwrap();
                // Get the reference field in tc_ref.
                let tc_addr = tc_ref.get("reference");
                if tc_addr.is_none() {
                    log::error!(
                        "Missing reference in tc_ref in DTP Sui Event message. workdir={} message={:?}",
                        self.params.workdir_name,
                        json_msg
                    );
                    return;
                }
                let tc_addr = tc_addr.unwrap().as_str();
                if tc_addr.is_none() {
                    log::error!(
                        "Invalid reference in tc_ref in DTP Sui Event message. workdir={} message={:?}",
                        self.params.workdir_name,
                        json_msg
                    );
                    return;
                }
                tc_addr.unwrap()
            } else {
                ""
            };

            // Process differently depending of the source
            // TODO Add Host support for when dtp_src == 3
            if dtp_src == 1 {
                let _rx_result = self
                    .handle_ws_msg_for_cli_ipipe(
                        subscription_number,
                        tc_addr,
                        dtp_src_addr,
                        parsed_json,
                    )
                    .await;
            } else if dtp_src == 2 {
                let _rx_result = self
                    .handle_ws_msg_for_srv_ipipe(
                        subscription_number,
                        tc_addr,
                        dtp_src_addr,
                        parsed_json,
                    )
                    .await;
            } else if dtp_src == 4 {
                let rx_result = self
                    .handle_ws_msg_for_package(subscription_number, result)
                    .await;
                if let Ok((package_uuid, package_name)) = rx_result {
                    let msg = GenericChannelMsg {
                        event_id: basic_types::EVENT_EXEC,
                        command: Some("add_sui_event".to_string()),
                        params: vec![package_uuid, package_name],
                        data_json: Some(json_msg.clone()),
                        workdir_idx: Some(self.params.workdir_idx),
                        resp_channel: None,
                    };
                    let ws_msg = WebSocketWorkerMsg::Generic(msg);
                    if self.params.parent_tx.send(ws_msg).await.is_err() {
                        let error_msg = format!(
                            "Failed to add_sui_event for workdir_idx={} message={:?}",
                            self.params.workdir_idx, json_msg
                        );
                        log::error!("{}", error_msg);
                    }
                    return;
                }
            }
        }
    }

    async fn handle_ws_msg_for_cli_ipipe(
        &mut self,
        subscription_number: u64,
        tc_id: &str,
        src_addr: &str,
        parsed_json: &Map<String, Value>,
    ) -> Result<(), anyhow::Error> {
        // If a matching request, forward the data into the one-shot response channel.
        // Consume the pending request.
        info!(
            "REQUEST Received subscription_number={} tc_id={} src_addr={} msg={:?}",
            subscription_number, tc_id, src_addr, parsed_json
        );
        Ok(())
    }

    async fn handle_ws_msg_for_srv_ipipe(
        &mut self,
        subscription_number: u64,
        tc_id: &str,
        src_addr: &str,
        parsed_json: &Map<String, Value>,
    ) -> Result<(), anyhow::Error> {
        // TODO Forward to an async TX thread to contact the server and respond back.
        // For now just reply back to the client directly here.
        info!(
            "RESPONSE Received subscription_number={} tc_id={} src_addr={} msg={:?}",
            subscription_number, tc_id, src_addr, parsed_json
        );
        Ok(())
    }

    async fn handle_ws_msg_for_package(
        &mut self,
        subscription_number: u64,
        result: &Map<String, Value>,
    ) -> Result<(String, String), anyhow::Error> {
        // Return the package_uuid and package_name if the message was recognized as
        // coming for a package subscription.
        //
        // Return Ok<None> if not related to a package subscription.
        // Try to find the related package uuid (Suibase ID) and name using the
        // subscription number.
        let mut package_uuid: Option<String> = None;
        let mut package_name: Option<String> = None;
        for package in self.package_subs.values_mut() {
            let state = package.state();
            if state == &SubscriptionTrackingState::Subscribed
                && package.subscription_number() == subscription_number
            {
                package_uuid = Some(package.uuid().clone());
                package_name = Some(package.name().clone());
                // While we are here... do a sanity check that packageId field
                // match what is in PackageTrackingState.
                let package_id = result["packageId"].as_str();
                if package_id.is_none() {
                    let error_msg = format!(
                        "Missing packageId in Sui Event message. workdir={}",
                        self.params.workdir_name,
                    );
                    log::error!("{}", error_msg);
                    bail!(error_msg);
                }
                let package_id = package_id.unwrap();
                // Verify package_id starts with "0x", and then create a slice that
                // remove the "0x".
                if !package_id.starts_with("0x") {
                    let error_msg = format!(
                        "Invalid packageId in Sui Event message. workdir={}",
                        self.params.workdir_name
                    );
                    log::error!("{}", error_msg);
                    bail!(error_msg);
                }
                let package_id = &package_id[2..];
                // Sanity test.
                let expected_package_id = package
                    .package_filter()
                    .cloned()
                    .unwrap_or_else(|| "".to_string());
                if package_id != expected_package_id {
                    let error_msg = format!(
                        "packageId {} not matching {} in Sui Event message. workdir={}",
                        package_id, expected_package_id, self.params.workdir_name,
                    );
                    log::error!("{}", error_msg);
                    bail!(error_msg);
                }
                break;
            }
        }

        if package_uuid.is_none() {
            let error_msg = format!(
                "Unsubscribed state for subscription number {} for Sui Event message. workdir={}",
                subscription_number, self.params.workdir_name,
            );
            log::warn!("{}", error_msg);
            bail!(error_msg);
        }
        let package_uuid = package_uuid.unwrap();

        if package_name.is_none() {
            let error_msg = format!(
                "Missing package name for subscription number {} for Sui Event message. workdir={}",
                subscription_number, self.params.workdir_name
            );
            log::warn!("{}", error_msg);
            bail!(error_msg);
        }
        let package_name = package_name.unwrap();

        // {"jsonrpc": String("2.0"),
        //  "method": String("suix_subscribeEvent"),
        //  "params": Object { "subscription": Number(6351273490251832),
        //                     "result": Object {
        //                        "id": Object {"txDigest": String("3Vua...ChrL"), "eventSeq": String("1")},
        //                        "packageId": String("0xe065...3b08"),
        //                        "transactionModule": String("Counter"),
        //                        "sender": String("0xf7ae...1462"),
        //                        "type": String("0xe065...3b08::Counter::CounterChanged"),
        //                        "parsedJson": Object {"by_address": String("0xf7ae...1462"), "count": String("1")},
        //                        "bcs": String("3t9dC...ELZ"),
        //                        "timestampMs": String("1703895010111")
        //                      }
        //                    }
        // }
        // TODO Validate here if from an expected subscribed package.
        // Forward to the parent thread for deduplication.
        Ok((package_uuid, package_name))
    }

    // Returns is_correlated_msg and trig_audit_event.
    fn tracker_update_state_correlation(
        tracker: &mut SubscriptionTracking,
        json_msg: &serde_json::Value,
        msg_seq_number: u64,
        workdir_name: &str,
    ) -> (bool, bool) {
        let mut is_correlated_msg = false;
        let mut trig_audit_event = false;
        let state = tracker.state();
        if state == &SubscriptionTrackingState::Subscribing {
            if tracker.did_sent_subscribe_request(msg_seq_number) {
                is_correlated_msg = true;
                log::info!(
                    "Received subscribe resp. workdir={} tracker={:?} resp={:?}",
                    workdir_name,
                    tracker,
                    json_msg,
                );
                // Got an expected subscribe response.
                // Extract the result string from the JSON message.
                let result = json_msg["result"].as_u64();
                if result.is_none() {
                    log::error!(
                                "Missing result field in subscribe JSON resp. workdir={} tracker={:?} resp={:?}",
                                workdir_name,
                                tracker,
                                json_msg
                            );
                    return (is_correlated_msg, trig_audit_event);
                }
                let unsubscribe_id = result.unwrap();
                tracker.report_subscribing_response(unsubscribe_id.to_string());
                trig_audit_event = true;
                return (is_correlated_msg, trig_audit_event);
            }
        } else if state == &SubscriptionTrackingState::Unsubscribing
            && tracker.did_sent_unsubscribe_request(msg_seq_number)
        {
            // Got an expected unsubscribe response.
            is_correlated_msg = true;
            log::info!(
                "Received unsubscribe resp. workdir={} tracker={:?} resp={:?}",
                workdir_name,
                tracker,
                json_msg,
            );

            tracker.report_unsubscribing_response();
            trig_audit_event = true;
        }
        (is_correlated_msg, trig_audit_event)
    }

    async fn tracker_state_update(
        tracker: &mut SubscriptionTracking,
        websocket: &mut WebSocketIOManagement,
    ) -> bool {
        let mut state_change = false;
        if tracker.is_remove_requested() {
            //log::info!("Initiating processing removed from package");
            if Self::try_to_unsubscribe(tracker, websocket).await {
                state_change = true;
            }
        } else {
            match tracker.state() {
                SubscriptionTrackingState::Disconnected => {
                    // Initial state.
                    if Self::try_to_subscribe(tracker, websocket).await {
                        state_change = true;
                    }
                }
                SubscriptionTrackingState::Subscribing => {
                    if Self::try_to_subscribe(tracker, websocket).await {
                        state_change = true;
                    }
                }
                SubscriptionTrackingState::Subscribed => {
                    // Nothing to do.
                    // Valid next states are Unsubscribing (removed from config) or Disconnected (on connection loss).
                }
                SubscriptionTrackingState::Unsubscribing => {
                    // Valid next state is Unsubscribed (on unsubscribed confirmation, timeout) and ReadyToDelete (on connection loss).
                    if Self::try_to_unsubscribe(tracker, websocket).await {
                        state_change = true;
                    }
                }
                SubscriptionTrackingState::ReadyToDelete => {
                    // End state. Nothing to do. The tracking will eventually be deleted on next audit.
                }
            }
        }
        state_change
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

        log::info!("Received an audit message: {:?}", msg);
        let mut state_change = false;
        {
            // Get a reader lock on the globals packages_config.
            let globals_read_guard = self.params.globals.packages_config.read().await;
            let workdirs = &globals_read_guard.workdirs;

            // Get the element in packages_config for workdir_idx.

            let move_configs =
                GlobalsPackagesConfigST::get_move_configs(workdirs, self.params.workdir_idx);
            // Note: it can be normal for move_configs to be none if the workdir had no package published yet.
            if move_configs.is_some() {
                let move_configs = move_configs.unwrap();

                // Check for adding PackagesTracking.
                // Add a PackagesTracking in the packages HashMap for every latests in packages_config.
                // Once created, the PackagesTracking remains until removed from packages_config.
                // The package_id is used as the key in the packages HashMap.
                for (uuid, move_config) in move_configs {
                    let latest = move_config.latest_package.as_ref().unwrap();
                    // Check if the package is already in the packages HashMap.
                    if !self.package_subs.contains_key(&latest.package_id) {
                        if move_config.path.is_none() {
                            log::error!("Missing path in move_config {:?}", move_config);
                            continue;
                        }
                        let toml_path = move_config.path.as_ref().unwrap().clone();

                        // Create a new PackagesTracking.
                        let package_tracking = SubscriptionTracking::new_for_managed_package(
                            toml_path,
                            latest.package_name.clone(),
                            uuid.to_string(),
                            latest.package_id.clone(),
                        );
                        // Add the PackagesTracking to the packages HashMap.
                        self.package_subs
                            .insert(latest.package_id.clone(), package_tracking);
                    }
                }

                // Transition package to Unsubscribing state when no longer in the config.
                // Remove the package tracking once unsubscription confirmed (or timeout).
                self.package_subs.retain(|package_id, package_tracking| {
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
            } // End of move_configs.is_some()
        } // End of globals packages_config read lock.

        // TODO Transition here to Disconnected or ReadyToDelete on connection lost?

        // Check to update every tracker state machine.
        let websocket = &mut self.websocket;
        let package_subs = &mut self.package_subs;
        for tracker in package_subs.values_mut() {
            if Self::tracker_state_update(tracker, websocket).await {
                state_change = true;
            }
        }

        let subs = &mut self.localhost_subs;
        for tracker in subs.values_mut() {
            if Self::tracker_state_update(tracker, websocket).await {
                state_change = true;
            }
        }
        /*
        let subs = &mut self.sender_subs;
        for tracker in subs.values_mut() {
            if Self::tracker_state_update(tracker, websocket).await {
                state_change = true;
            }
        }*/

        let cli_conns = &mut self.cli_conns;
        for tracker in cli_conns.values_mut() {
            for ipipe in tracker.ipipe_trackings.values_mut() {
                if Self::tracker_state_update(&mut ipipe.subs, websocket).await {
                    state_change = true;
                }
            }
        }
        let srv_conns = &mut self.srv_conns;
        for tracker in srv_conns.values_mut() {
            for ipipe in tracker.ipipe_trackings.values_mut() {
                if Self::tracker_state_update(&mut ipipe.subs, websocket).await {
                    state_change = true;
                }
            }
        }

        if state_change {
            // Update the packages_config globals.
            let generic_msg = GenericChannelMsg {
                event_id: basic_types::EVENT_UPDATE,
                command: None,
                params: Vec::new(),
                data_json: None,
                workdir_idx: Some(self.params.workdir_idx),
                resp_channel: None,
            };
            let ws_io_msg = WebSocketWorkerIOMsg::Generic(generic_msg);
            if self.params.self_tx.send(ws_io_msg).await.is_err() {
                log::error!(
                    "Failed to send update message for workdir_idx={}",
                    self.params.workdir_idx
                );
            }
        }
    }

    async fn send_audit_msg_to_self(&self) {
        let generic_msg = GenericChannelMsg {
            event_id: basic_types::EVENT_AUDIT,
            command: None,
            params: Vec::new(),
            data_json: None,
            workdir_idx: Some(self.params.workdir_idx),
            resp_channel: None,
        };
        let ws_io_msg = WebSocketWorkerIOMsg::Generic(generic_msg);
        if self.params.self_tx.send(ws_io_msg).await.is_err() {
            log::error!(
                "Failed to send audit self-message for workdir_idx={}",
                self.params.workdir_idx
            );
        }
    }

    async fn process_update_msg(&mut self, msg: GenericChannelMsg) {
        // This function takes care of synching between self.package_subs
        // and global packages_config.
        //
        // Unlike an audit, changes to packages_config globals are
        // allowed here.
        //log::info!("Received an update message: {:?}", msg);

        // TODO For robustness, implement similar global<->self.localhost_subs and global<->self.conns
        //      For now localhost_subs&conns are updated with one-time msg (e.g process_update_localhost).

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
                if !self.package_subs.contains_key(&latest.package_id) {
                    if move_config.path.is_none() {
                        log::error!("Missing path in move_config {:?}", move_config);
                        continue;
                    }
                    let toml_path = move_config.path.as_ref().unwrap().clone();

                    // Create a new PackagesTracking.
                    let package_tracking = SubscriptionTracking::new_for_managed_package(
                        toml_path,
                        latest.package_name.clone(),
                        uuid.to_string(),
                        latest.package_id.clone(),
                    );
                    // Add the PackagesTracking to the packages HashMap.
                    self.package_subs
                        .insert(latest.package_id.clone(), package_tracking);
                    trig_audit = true;
                } else {
                    let package_tracking = &self.package_subs[&latest.package_id];
                    let package_tracking_state: u32 = package_tracking.state().clone().into();
                    if move_config.tracking_state != package_tracking_state {
                        move_config.tracking_state = package_tracking_state;
                    }
                }
            }
        }

        if trig_audit {
            self.send_audit_msg_to_self().await;
        }
    }

    async fn process_localhost_update(&mut self, msg: ExtendedWebSocketWorkerIOMsg) {
        // Create an instance of SubscriptionTracking and add
        // it to self.localhost_subs, but only if the key msg.localhost.object_id()
        // is not already in self.localhost_subs.

        // This is similar to process_update_msg(), except that it
        // handles only one host specified in the message (instead of
        // getting the info from the globals).

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.generic.workdir_idx {
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

        if msg.localhost.is_none() {
            log::error!("process_localhost_update - Missing localhost parameter");
            return;
        }

        if msg.package.is_none() {
            log::error!("process_localhost_update - Missing package parameter");
            return;
        }

        if let Some(localhost) = msg.localhost {
            let object_id = localhost.object_id();
            let package_id = localhost.package_id();
            if !self.localhost_subs.contains_key(object_id) {
                let localhost_tracking = SubscriptionTracking::new(
                    package_id.to_string(),
                    Some(object_id.to_string()),
                    None,
                );
                self.localhost_subs
                    .insert(*localhost.object_id(), localhost_tracking);
                self.send_audit_msg_to_self().await;
            }
        }
    }
    /*
    async fn process_sender_update(&mut self, msg: ExtendedWebSocketWorkerIOMsg) {
        // Create an instance of SubscriptionTracking and add
        // it to one of self.sender_subs.
        //
        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.generic.workdir_idx {
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

        if msg.package.is_none() {
            log::error!("process_sender_update - Missing package parameter",);
            return;
        }
        let package = msg.package.unwrap();

        if msg.sender.is_none() {
            log::error!("process_sender_update - Missing sender parameter");
            return;
        }
        let sender = msg.sender.unwrap();

        let key = (package, sender);
        if !self.sender_subs.contains_key(&key) {
            let tracker =
                SubscriptionTracking::new_for_package_sender(key.0.clone(), key.1.clone());
            self.sender_subs.insert(key, tracker);
            self.send_audit_msg_to_self().await;
        }
    }
    */

    async fn process_conn_update(&mut self, msg: ExtendedWebSocketWorkerIOMsg) {
        // Create an instance of SubscriptionTracking for each new pipes/ipipes
        //
        // This is similar to process_update_msg(), except that it
        // handles only the objects specified in the message (instead of
        // getting the info from the globals).

        // Verify that the workdir_idx is as expected.

        if let Some(workdir_idx) = msg.generic.workdir_idx {
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

        if msg.package.is_none() {
            log::error!("process_conn_update - Missing package parameter");
            return;
        }
        let package_id = msg.package.unwrap();

        if msg.conn.is_none() {
            log::error!("process_conn_update - Missing Connection parameter");
            return;
        }
        let connection = msg.conn.unwrap();

        let conn_obj = connection.get_conn_objects().await;
        if conn_obj.is_none() {
            log::error!("process_conn_update - Missing Connection.conn_objects");
            return;
        }
        let conn_objs = conn_obj.unwrap();

        let mut trigger_audit = false;

        let tc_object_id = conn_objs.tc.to_string();

        if !conn_objs.cli_tx_ipipes.is_empty() {
            // Make sure the tc_object_id is in self.cli_conns then adds
            // the individual ipipes to the InnerPipesTracking (if not already there).
            if !self.cli_conns.contains_key(&tc_object_id) {
                let mut conn_tracking = ClientConnTracking::default();
                for ipipe_object_id in conn_objs.cli_tx_ipipes.iter() {
                    let ipipe_addr = ipipe_object_id.to_string();
                    let tracker = SubscriptionTracking::new(
                        package_id.clone(),
                        Some(ipipe_addr.clone()),
                        Some(conn_objs.cli_auth.to_string()),
                    );
                    conn_tracking
                        .ipipe_trackings
                        .insert(ipipe_addr, InnerPipeTracking { subs: tracker });
                }
                self.cli_conns
                    .insert(tc_object_id.to_string(), conn_tracking);
                trigger_audit = true;
            } else {
                // Just add the conn.cli_tx_ipipes not in self.cli_conns
                let conn_tracking = self.cli_conns.get_mut(&tc_object_id).unwrap();

                for ipipe_object_id in conn_objs.cli_tx_ipipes.iter() {
                    let ipipe_addr = ipipe_object_id.to_string();
                    if !conn_tracking.ipipe_trackings.contains_key(&ipipe_addr) {
                        let tracker = SubscriptionTracking::new(
                            package_id.clone(),
                            Some(ipipe_addr.clone()),
                            Some(conn_objs.cli_auth.to_string()),
                        );
                        conn_tracking
                            .ipipe_trackings
                            .insert(ipipe_addr, InnerPipeTracking { subs: tracker });
                        trigger_audit = true;
                    }
                }
            }
        }

        // TODO Refactor this... same pattern as the code above.
        if !conn_objs.srv_tx_ipipes.is_empty() {
            // Make sure the tc_object_id is in self.cli_conns then adds
            // the individual ipipes to the InnerPipesTracking (if not already there).
            if !self.srv_conns.contains_key(&tc_object_id) {
                let mut conn_tracking = ServerConnTracking::default();
                for ipipe_object_id in conn_objs.srv_tx_ipipes.iter() {
                    let ipipe_addr = ipipe_object_id.to_string();
                    let tracker = SubscriptionTracking::new(
                        package_id.clone(),
                        Some(ipipe_addr.clone()),
                        Some(conn_objs.srv_auth.to_string()),
                    );
                    conn_tracking
                        .ipipe_trackings
                        .insert(ipipe_addr, InnerPipeTracking { subs: tracker });
                }
                self.srv_conns
                    .insert(tc_object_id.to_string(), conn_tracking);
                trigger_audit = true;
            } else {
                // Just add the conn.srv_tx_ipipes not in self.srv_conns
                let conn_tracking = self.srv_conns.get_mut(&tc_object_id).unwrap();
                for ipipe_object_id in conn_objs.srv_tx_ipipes.iter() {
                    let ipipe_addr = ipipe_object_id.to_string();
                    if !conn_tracking.ipipe_trackings.contains_key(&ipipe_addr) {
                        let tracker = SubscriptionTracking::new(
                            package_id.clone(),
                            Some(ipipe_addr.clone()),
                            Some(conn_objs.srv_auth.to_string()),
                        );
                        conn_tracking
                            .ipipe_trackings
                            .insert(ipipe_addr, InnerPipeTracking { subs: tracker });
                        trigger_audit = true;
                    }
                }
            }
        }

        if trigger_audit {
            self.send_audit_msg_to_self().await;
        }
    }

    async fn try_to_subscribe(
        tracker: &mut SubscriptionTracking,
        websocket: &mut WebSocketIOManagement,
    ) -> bool {
        // Send a subscribe message, unless there is one already recently pending.
        // On failure, keep retrying as long that package is configured.
        // (retry will be on subsequent call).
        //
        // Return true if there is a state change.
        let mut state_change = false;
        match tracker.state() {
            SubscriptionTrackingState::Disconnected => {
                // Valid state when calling this function.
                if tracker.change_state_to(SubscriptionTrackingState::Subscribing) {
                    state_change = true;
                }
            }
            SubscriptionTrackingState::Subscribing => {
                if tracker.unsubscribed_id().is_some() {
                    if tracker.change_state_to(SubscriptionTrackingState::Subscribed) {
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
        if tracker.secs_since_last_request() < 2 {
            send_subscribe_message = false;
        }

        if send_subscribe_message {
            if tracker.package_filter().is_none() {
                log::error!("Missing package_filter in SubscriptionTracking");
                return false;
            }
            let package_id = tracker.package_filter().cloned().unwrap_or_default();

            // Check if retrying and log error only on first retry and once in a while after.
            if tracker.request_retry() % 3 == 1 {
                log::error!("Failed to subscribe package_id={:?}", package_id);
            }

            websocket.seq_number += 1;
            tracker.report_subscribing_request(websocket.seq_number);
            let msg = Message::Text(Self::subscribe_request_format(
                websocket.seq_number,
                &package_id, // Must not have leading 0x
                tracker.src_addr_filter(),
                tracker.sender_filter(),
            ));

            if let Some(ref mut write) = websocket.write {
                //log::info!("Sending subscribe message: {:?}", msg);
                // 'send' is equivalent to call write+flush.
                // https://docs.rs/tungstenite/latest/tungstenite/protocol/struct.WebSocket.html#method.send
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
        tracker: &mut SubscriptionTracking,
        websocket: &mut WebSocketIOManagement,
    ) -> bool {
        // If subscribed, then send a unsubscribe message, unless there is one
        // already recently pending.
        //
        // On failure, keep retrying until timeout (retry will be on subsequent call).
        // After being confirmed unsubscribe (or timeout) the PackageTracking state
        // becomes ReadyToDelete.
        let mut state_change = false;
        match tracker.state() {
            SubscriptionTrackingState::Disconnected => {
                // No subscription on-going...
                if tracker.change_state_to(SubscriptionTrackingState::ReadyToDelete) {
                    state_change = true;
                }
                return state_change;
            }
            SubscriptionTrackingState::Subscribing => {
                // If trying to unsubscribe while a subscription request was already sent (and
                // no response receive yet), then let the subscription a chance to complete.
                // This will allow for a clean unsubscribe later.
                // Check for a subscription timeout transition to avoid being block forever.
                if tracker.is_subscribe_request_pending_response()
                    && tracker.secs_since_last_request() >= 2
                {
                    // Do nothing... to give a chance for the subscription to succeed.
                    state_change = false;
                    return state_change;
                }

                if tracker.change_state_to(SubscriptionTrackingState::Unsubscribing) {
                    state_change = true;
                }
                return state_change;
            }
            SubscriptionTrackingState::Subscribed => {
                if tracker.change_state_to(SubscriptionTrackingState::Unsubscribing) {
                    state_change = true;
                }
                return state_change;
            }

            SubscriptionTrackingState::Unsubscribing => {
                // Ready to delete if unsubscribed_id is clear or timeout.
                // The unsubscribed_id is clear when receiving a unsubscribe response.
                if tracker.unsubscribed_id().is_none() || tracker.request_retry() > 10 {
                    if tracker.change_state_to(SubscriptionTrackingState::ReadyToDelete) {
                        state_change = true;
                    }
                    return state_change;
                }
            }

            SubscriptionTrackingState::ReadyToDelete => {
                // Nothing to do.
                state_change = false;
                return state_change;
            }
        };

        // If there is no known unsubscribed_id, then no point to try to unsubscribe.
        if tracker.unsubscribed_id().is_none() {
            if tracker.change_state_to(SubscriptionTrackingState::ReadyToDelete) {
                state_change = true;
            }
            return state_change;
        }

        let mut send_unsubscribe_message = true;
        // Don't do it if one was already sent in last 2 seconds.
        if tracker.secs_since_last_request() < 2 {
            send_unsubscribe_message = false;
        }

        if send_unsubscribe_message {
            // Periodically report an error on too many retry.
            if tracker.request_retry() % 3 == 1 {
                log::error!("Failed to unsubscribe");
            }
            websocket.seq_number += 1;
            tracker.report_unsubscribing_request(websocket.seq_number);
            let msg = Message::Text(Self::unsubscribe_request_format(
                websocket.seq_number,
                tracker.unsubscribed_id().unwrap(), // Must not have leading 0x
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

        // For now, the only tested servers for websocket are Shinami
        // and Mysten Labs.

        // TODO: Make this better config driven.

        // Get the InputPort config from the globals.proxy (read-only).
        let globals_read_guard = self.params.globals.proxy.read().await;
        let globals = &*globals_read_guard;
        let input_ports = &globals.input_ports;

        // Iterate input_ports to find a matching workdir_idx.
        let configured_rpc = input_ports.iter().find_map(|x| {
            let (_, input_port) = x;
            if input_port.workdir_idx() == self.params.workdir_idx {
                input_port.find_target_server_ws_by_alias("shinami.com")
            } else {
                None
            }
        });

        let socket_url = if let Some(configured_rpc) = configured_rpc {
            configured_rpc
        } else {
            let default_rpc = match self.params.workdir_idx {
                WORKDIR_IDX_LOCALNET => "ws://0.0.0.0:9000",
                WORKDIR_IDX_DEVNET => "wss://fullnode.devnet.sui.io:443",
                WORKDIR_IDX_TESTNET => "wss://fullnode.testnet.sui.io:443",
                WORKDIR_IDX_MAINNET => "wss://fullnode.mainnet.sui.io:443",
                _ => {
                    log::error!("Unexpected workdir_idx {:?}", self.params.workdir_idx);
                    return false;
                }
            };
            default_rpc.to_string()
        };

        match connect_async(&socket_url).await {
            Ok((ws_stream, _response)) => {
                let (write, read) = ws_stream.split();
                self.websocket.write = Some(write);
                self.websocket.read = Some(read);
            }
            Err(e) => {
                // Display only once.
                log::error!("connect_async error: {:?} to {:?}", e, &socket_url);
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
                        if let Ok(msg) = msg {
                            // Process the message.
                            self.process_ws_msg(msg).await;
                        } else {
                            // Connection lost.
                            //log::info!("Connection lost for {}", self.params.workdir_name);
                            self.websocket.write = None;
                            self.websocket.read = None;
                            return;
                        }
                    } else {
                        // Shutdown requested.
                        log::info!("Received {} None websocket message", self.params.workdir_name);
                        return;
                    }
                }
                msg = event_rx_future => {
                    if let Some(ws_io_msg) = msg {
                        match ws_io_msg {
                            WebSocketWorkerIOMsg::Generic(generic_msg) => {
                                match generic_msg.event_id {
                                    basic_types::EVENT_AUDIT => {
                                        self.process_audit_msg(generic_msg).await;
                                    },
                                    basic_types::EVENT_UPDATE => {
                                        self.process_update_msg(generic_msg).await;
                                    },
                                    _ => {
                                        // Consume unexpected messages.
                                        log::error!("Unexpected event_id {:?}", generic_msg );
                                    }
                                }
                            },
                            WebSocketWorkerIOMsg::Extended(extended_msg) => {
                                match extended_msg.generic.event_id {
                                    basic_types::EVENT_EXEC => {
                                        match extended_msg.generic.command.as_deref() {
                                            Some("localhost_update") => {
                                                self.process_localhost_update(extended_msg).await;
                                            },
                                            Some("conn_update") => {
                                                self.process_conn_update(extended_msg).await;
                                            },
                                            _ => {
                                                // Consume unexpected messages.
                                                log::error!("Unexpected extended generic.command {:?}", extended_msg );
                                            }
                                        }
                                    },
                                    _ => {
                                        // Consume unexpected messages.
                                        log::error!("Unexpected extended generic.event_id {:?}", extended_msg );
                                    }
                                }
                            },
                        }
                    } else {
                        // Channel closed or shutdown requested.
                        log::info!("Received {} None internal message", self.params.workdir_name );
                        return;
                    }
                }
            }
        }
    }
}
