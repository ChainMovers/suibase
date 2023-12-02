// Dedup all Sui events for a single workdir. The dedup results are written to SQLite.
//
// This thread process the data coming from one (may be more later) events_worker child.
//
// The events_worker is responsible to subscribe/unsubscribe events, filter them
// and forward the validated data to its events_writer_worker parent.
use std::sync::Arc;

use crate::{
    basic_types::{self, AutoThread, GenericChannelMsg, GenericRx, Runnable, WorkdirIdx},
    shared_types::Globals,
    workers::{DBWorker, DBWorkerParams, WebSocketWorker, WebSocketWorkerParams},
};

use anyhow::Result;
use axum::async_trait;

use tokio::sync::{mpsc::Sender, Mutex};
use tokio_graceful_shutdown::{FutureExt, SubsystemBuilder, SubsystemHandle};

#[derive(Clone)]
pub struct EventsWriterWorkerParams {
    globals: Globals,
    event_rx: Arc<Mutex<GenericRx>>,
    workdir_idx: WorkdirIdx,
}

impl EventsWriterWorkerParams {
    pub fn new(globals: Globals, event_rx: GenericRx, workdir_idx: WorkdirIdx) -> Self {
        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            workdir_idx,
        }
    }
}

pub struct EventsWriterWorker {
    auto_thread: AutoThread<EventsWriterThread, EventsWriterWorkerParams>,
}

impl EventsWriterWorker {
    pub fn new(params: EventsWriterWorkerParams) -> Self {
        Self {
            auto_thread: AutoThread::new("EventsWriter".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct EventsWriterThread {
    name: String,
    params: EventsWriterWorkerParams,
    ws_workers_channel: Vec<Sender<GenericChannelMsg>>,
    db_worker_channel: Option<Sender<GenericChannelMsg>>,
}

#[async_trait]
impl Runnable<EventsWriterWorkerParams> for EventsWriterThread {
    fn new(name: String, params: EventsWriterWorkerParams) -> Self {
        Self {
            name,
            params,
            ws_workers_channel: Vec::new(),
            db_worker_channel: None,
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        // Start a child websocket_worker thread.
        let (worker_tx, worker_rx) = tokio::sync::mpsc::channel(1000);
        let ws_worker_params = WebSocketWorkerParams::new(
            self.params.globals.clone(),
            worker_rx,
            worker_tx.clone(),
            self.params.workdir_idx,
        );
        let ws_worker = WebSocketWorker::new(ws_worker_params);
        subsys.start(SubsystemBuilder::new("ws-worker", |a| ws_worker.run(a)));
        self.ws_workers_channel.push(worker_tx);

        // Start a child db_worker thread.
        let (db_worker_tx, db_worker_rx) = tokio::sync::mpsc::channel(1000);
        let db_worker_params = DBWorkerParams::new(
            self.params.globals.clone(),
            db_worker_rx,
            db_worker_tx.clone(),
            self.params.workdir_idx,
        );
        let db_worker = DBWorker::new(db_worker_params);
        subsys.start(SubsystemBuilder::new("db-worker", |a| db_worker.run(a)));
        self.db_worker_channel = Some(db_worker_tx);

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

impl EventsWriterThread {
    async fn forward_to_children(&mut self, msg: GenericChannelMsg) {
        // Forward the message to each self.ws_workers_channel.
        for tx in &self.ws_workers_channel {
            let forward_msg = GenericChannelMsg {
                event_id: msg.event_id,
                data_string: msg.data_string.clone(),
                workdir_idx: msg.workdir_idx,
                resp_channel: None,
            };
            let _ = tx.send(forward_msg).await;
        }
        // Forward the message to the single self.db_worker_channel.
        if let Some(tx) = &self.db_worker_channel {
            let forward_msg = GenericChannelMsg {
                event_id: msg.event_id,
                data_string: msg.data_string.clone(),
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
