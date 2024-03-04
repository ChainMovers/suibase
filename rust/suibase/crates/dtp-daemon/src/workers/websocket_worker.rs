// Handle all Websocket TX/RX for a single workdir.
//
// There are sub-threads involve because of various blocking events.
// This thread is responsible for restarting the sub-threads if they die.
//
// Sub-threads:
//   WebSocketWorkerIO:  Lowest level thread doing the RX/TX with the Sui network.
//                       It also handle WebSocket reconnection/recovery and
//                       subscribe/unsubscribe tracking.
//
//   WebSocketWorkerTX: Serialize the transmission of data toward the WebSocketWorkerIO.
//                      Handles the "request" side of a RPC/query.
//
//   WebSocketWorkerRX: Serialize the reception of data from the WebSocketWorkerIO.
//                      Handles the "response" side of a RPC/query.
//
//   DBWorker: Some data received are directed to be written to the DB.
//

//
use std::sync::Arc;

use crate::{
    shared_types::{self, Globals},
    workers::{DBWorker, DBWorkerParams, WebSocketWorkerIO, WebSocketWorkerIOParams},
};

use common::basic_types::{
    self, AutoThread, GenericChannelMsg, GenericRx, GenericTx, Runnable, WorkdirIdx,
};

use anyhow::Result;
use axum::async_trait;

use tokio::sync::{mpsc::Sender, Mutex};
use tokio_graceful_shutdown::{FutureExt, SubsystemBuilder, SubsystemHandle};

#[derive(Clone)]
pub struct WebSocketWorkerParams {
    globals: Globals,
    event_rx: Arc<Mutex<GenericRx>>,
    event_tx: GenericTx,
    workdir_idx: WorkdirIdx,
    workdir_name: String,
}

impl WebSocketWorkerParams {
    pub fn new(
        globals: Globals,
        event_rx: GenericRx,
        event_tx: GenericTx,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        // For now, support only built-in workdirs ("localnet", "testnet"...).
        let workdir_name = shared_types::WORKDIRS_KEYS[workdir_idx as usize].to_string();

        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            event_tx,
            workdir_idx,
            workdir_name,
        }
    }
}

pub struct WebSocketWorker {
    auto_thread: AutoThread<WebSocketThread, WebSocketWorkerParams>,
}

impl WebSocketWorker {
    pub fn new(params: WebSocketWorkerParams) -> Self {
        Self {
            auto_thread: AutoThread::new("EventsWriter".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct WebSocketThread {
    name: String,
    params: WebSocketWorkerParams,
    ws_workers_channel: Vec<Sender<GenericChannelMsg>>,
    db_worker_channel: Option<Sender<GenericChannelMsg>>,
}

#[async_trait]
impl Runnable<WebSocketWorkerParams> for WebSocketThread {
    fn new(name: String, params: WebSocketWorkerParams) -> Self {
        Self {
            name,
            params,
            ws_workers_channel: Vec::new(),
            db_worker_channel: None,
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started for {}", self.params.workdir_name);

        // Start a child websocket_worker thread (in future, more than one might be started).
        let (worker_tx, worker_rx) = tokio::sync::mpsc::channel(1000);
        let ws_worker_params = WebSocketWorkerIOParams::new(
            self.params.globals.clone(),
            worker_rx,
            worker_tx.clone(),
            self.params.event_tx.clone(),
            self.params.workdir_idx,
        );
        let ws_worker = WebSocketWorkerIO::new(ws_worker_params);
        subsys.start(SubsystemBuilder::new("ws-worker", |a| ws_worker.run(a)));
        self.ws_workers_channel.push(worker_tx);

        // Start a single child db_worker thread.
        let (db_worker_tx, db_worker_rx) = tokio::sync::mpsc::channel(1000);
        let db_worker_params = DBWorkerParams::new(
            self.params.globals.clone(),
            db_worker_rx,
            db_worker_tx.clone(),
            self.params.workdir_idx,
            self.params.workdir_name.clone(),
        );
        let db_worker = DBWorker::new(db_worker_params);
        subsys.start(SubsystemBuilder::new("db-worker", |a| db_worker.run(a)));
        self.db_worker_channel = Some(db_worker_tx);

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

impl WebSocketThread {
    async fn forward_to_children(&mut self, msg: GenericChannelMsg) {
        // Forward the message to each self.ws_workers_channel.
        for tx in &self.ws_workers_channel {
            let forward_msg = GenericChannelMsg {
                event_id: msg.event_id,
                command: msg.command.clone(),
                params: msg.params.clone(),
                data_json: msg.data_json.clone(),
                workdir_idx: msg.workdir_idx,
                resp_channel: None,
            };
            let _ = tx.send(forward_msg).await;
        }
        // Forward the message to the single self.db_worker_channel.
        self.forward_to_db_worker(msg).await;
    }

    async fn forward_to_db_worker(&mut self, msg: GenericChannelMsg) {
        // Forward the message to the single self.db_worker_channel.
        if let Some(tx) = &self.db_worker_channel {
            let forward_msg = GenericChannelMsg {
                event_id: msg.event_id,
                command: msg.command,
                params: msg.params,
                data_json: msg.data_json,
                workdir_idx: msg.workdir_idx,
                resp_channel: None,
            };
            let _ = tx.send(forward_msg).await;
        }
    }

    async fn process_audit_msg(&mut self, msg: GenericChannelMsg) {
        self.forward_to_children(msg).await;
    }

    async fn process_update_msg(&mut self, msg: GenericChannelMsg) {
        self.forward_to_children(msg).await;
    }

    async fn process_add_sui_event(&mut self, msg: GenericChannelMsg) {
        // TODO Dedup logic to be done here, for now just forward everything since
        //      suibase currently support only a single websocket worker per workdir.
        self.forward_to_db_worker(msg).await;
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // Take mutable ownership of the event_rx channel as long this thread is running.
        let event_rx = Arc::clone(&self.params.event_rx);
        let mut event_rx = event_rx.lock().await;

        while !subsys.is_shutdown_requested() {
            // Wait for a suibase internal message (not a websocket message!).
            if let Some(msg) = event_rx.recv().await {
                // Process the message.
                match msg.event_id {
                    basic_types::EVENT_AUDIT => {
                        self.process_audit_msg(msg).await;
                    }
                    basic_types::EVENT_UPDATE => {
                        self.process_update_msg(msg).await;
                    }
                    basic_types::EVENT_EXEC => {
                        if let Some(command) = msg.command() {
                            if command == "add_sui_event" {
                                self.process_add_sui_event(msg).await;
                            } else {
                                log::error!(
                                    "Received a EVENT_EXEC message with unexpected command {}",
                                    command
                                );
                            }
                        } else {
                            log::error!("Received a EVENT_EXEC message without command");
                        }
                    }
                    _ => {
                        // Consume unexpected messages.
                        log::error!("Unexpected event_id {:?}", msg);
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
