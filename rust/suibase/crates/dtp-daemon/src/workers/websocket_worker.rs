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
    shared_types::{Globals, WebSocketWorkerMsg, WebSocketWorkerRx, WebSocketWorkerTx},
    workers::{WebSocketWorkerIO, WebSocketWorkerIOParams},
};

use common::basic_types::{
    self, AutoSizeVecMapVec, AutoThread, GenericChannelMsg, Runnable, WorkdirIdx,
};

use anyhow::Result;
use axum::async_trait;

use tokio::sync::{mpsc::Sender, Mutex};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle};

#[derive(Clone)]
pub struct WebSocketWorkerParams {
    globals: Globals,
    event_rx: Arc<Mutex<WebSocketWorkerRx>>,
    self_tx: WebSocketWorkerTx,
    workdir_idx: WorkdirIdx,
    workdir_name: String,
}

impl WebSocketWorkerParams {
    pub fn new(
        globals: Globals,
        event_rx: WebSocketWorkerRx,
        event_tx: WebSocketWorkerTx,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        // For now, support only built-in workdirs ("localnet", "testnet"...).
        let workdir_name = common::shared_types::WORKDIRS_KEYS[workdir_idx as usize].to_string();

        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            self_tx: event_tx,
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

#[derive(Default)]
struct WebSocketSubThread {
    is_running: bool,
    channel: Option<Sender<GenericChannelMsg>>,
}

struct WebSocketThread {
    name: String,
    params: WebSocketWorkerParams,
    worker_io: Option<WebSocketSubThread>,
    db_worker: Option<WebSocketSubThread>,
    workers_tx: AutoSizeVecMapVec<WebSocketSubThread>,
    workers_rx: AutoSizeVecMapVec<WebSocketSubThread>,
}

#[async_trait]
impl Runnable<WebSocketWorkerParams> for WebSocketThread {
    fn new(name: String, params: WebSocketWorkerParams) -> Self {
        Self {
            name,
            params,
            worker_io: None,
            db_worker: None,
            workers_tx: AutoSizeVecMapVec::new(),
            workers_rx: AutoSizeVecMapVec::new(),
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started for {}", self.params.workdir_name);

        // For now, just start a single instance of each SubThread.

        let (worker_io_tx, worker_io_rx) = tokio::sync::mpsc::channel(1000);
        let (worker_tx_tx, _worker_tx_rx) = tokio::sync::mpsc::channel(1000);
        let (worker_rx_tx, _worker_rx_rx) = tokio::sync::mpsc::channel(1000);

        // Add a reference to all TX channels into the globals.
        {
            let mut channels_guard = self
                .params
                .globals
                .get_channels(self.params.workdir_idx)
                .write()
                .await;
            let channels = &mut *channels_guard;
            channels.to_websocket_worker = Some(self.params.self_tx.clone());
            channels.to_websocket_worker_tx = Some(worker_tx_tx.clone());
            channels.to_websocket_worker_rx = Some(worker_rx_tx.clone());
        }

        // Remember the channels for sub-treads.

        // Start a child io thread. This is the actual WebSocket to the outside world.
        {
            let ws_worker_params = WebSocketWorkerIOParams::new(
                self.params.globals.clone(),
                worker_io_rx,
                worker_io_tx,
                self.params.self_tx.clone(),
                self.params.workdir_idx,
            );
            let ws_worker = WebSocketWorkerIO::new(ws_worker_params);
            subsys.start(SubsystemBuilder::new("ws-worker-io", |a| ws_worker.run(a)));
            self.worker_io = Some(WebSocketSubThread {
                is_running: true,
                channel: Some(worker_tx_tx),
            });
        }

        // TODO Implement TX/RX threads.

        // Start a single child db_worker thread.
        /* Not applicable for now
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
        }*/
        Ok(())
    }
}

impl WebSocketThread {
    async fn forward_to_children(&mut self, msg: GenericChannelMsg) {
        // TODO Forward to all children.
        // Forward the message to the worker_io
        self.forward_to_worker_io(msg).await;
    }

    /*
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
    }*/

    async fn forward_to_worker_io(&mut self, msg: GenericChannelMsg) {
        // Forward the message to the single self.worker_io.channel.
        if let Some(worker) = &self.worker_io {
            if let Some(tx) = &worker.channel {
                let forward_msg = GenericChannelMsg {
                    event_id: msg.event_id,
                    command: msg.command,
                    params: msg.params,
                    data_json: msg.data_json,
                    workdir_idx: msg.workdir_idx,
                    resp_channel: msg.resp_channel,
                };
                let _ = tx.send(forward_msg).await;
            }
        }
    }

    async fn process_audit_msg(&mut self, msg: GenericChannelMsg) {
        self.forward_to_children(msg).await;
    }

    async fn process_update_msg(&mut self, msg: GenericChannelMsg) {
        self.forward_to_children(msg).await;
    }

    async fn process_dtp_open_conn(&mut self, msg: GenericChannelMsg) {
        // TODO Forward instead to a workers_tx once implemented.
        self.forward_to_worker_io(msg).await;
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // Take mutable ownership of the event_rx channel as long this thread is running.
        let event_rx = Arc::clone(&self.params.event_rx);
        let mut event_rx = event_rx.lock().await;

        while !subsys.is_shutdown_requested() {
            // Wait for a suibase internal message (not a websocket message!).
            if let Some(msg) = event_rx.recv().await {
                match msg {
                    WebSocketWorkerMsg::Generic(msg) => {
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
                                    if command == "dtp_open_conn" {
                                        self.process_dtp_open_conn(msg).await;
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
