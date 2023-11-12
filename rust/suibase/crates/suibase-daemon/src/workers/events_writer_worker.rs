// Dedup all Sui events for a single workdir. The dedup results are written to SQLite.
//
// This thread process the data coming from one (may be more later) events_worker child.
//
// The events_worker is responsible to subscribe/unsubscribe events, filter them
// and forward the validated data to its events_writer_worker parent.
use std::{collections::HashSet, process::Command, sync::Arc};

use crate::{
    admin_controller::{self, AdminControllerMsg, AdminControllerRx},
    basic_types::{AutoThread, Runnable, WorkdirIdx},
    shared_types::Globals,
    workers::{WebSocketWorker, WebSocketWorkerParams},
};

use anyhow::Result;
use axum::async_trait;

use tokio::sync::Mutex;
use tokio_graceful_shutdown::{FutureExt, SubsystemBuilder, SubsystemHandle};

#[derive(Clone)]
pub struct EventsWriterWorkerParams {
    _globals: Globals,
    event_rx: Arc<Mutex<AdminControllerRx>>,
    workdir_idx: Option<WorkdirIdx>,
}

impl EventsWriterWorkerParams {
    pub fn new(
        globals: Globals,
        event_rx: AdminControllerRx,
        workdir_idx: Option<WorkdirIdx>,
    ) -> Self {
        Self {
            _globals: globals,
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
    // Set of unique packaged id (string).
    subscribed_ids: HashSet<String>,

    // Last known valid sequence number processed.
    last_seq_number: u64,
}

#[async_trait]
impl Runnable<EventsWriterWorkerParams> for EventsWriterThread {
    fn new(name: String, params: EventsWriterWorkerParams) -> Self {
        Self {
            name,
            params,
            subscribed_ids: HashSet::new(),
            last_seq_number: 0,
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        // Start a child websocket_worker thread.
        let (_worker_tx, worker_rx) = tokio::sync::mpsc::channel(1000);
        let ws_worker_params = WebSocketWorkerParams::new(
            self.params._globals.clone(),
            worker_rx,
            self.params.workdir_idx,
        );
        let ws_worker = WebSocketWorker::new(ws_worker_params);
        subsys.start(SubsystemBuilder::new("ws-worker", |a| ws_worker.run(a)));
        // TODO Send a periodic audit message to the websocket_worker.

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
    async fn do_exec(&mut self, msg: AdminControllerMsg) {
        // No error return here. Once the execution is completed, the output
        // of the response is returned to requester with a one shot message.
        //
        // If the response starts with "Error:", then an error was detected.
        //
        // Some effects are also possible on globals, particularly
        // for sharing large results.
        //
        log::info!(
            "do_exec() msg {:?} for workdir_idx={:?}",
            msg,
            self.params.workdir_idx
        );

        let resp = if msg.event_id != admin_controller::EVENT_SHELL_EXEC {
            log::error!("Unexpected event_id {:?}", msg.event_id);
            format!("Error: Unexpected event_id {:?}", msg.event_id)
        } else if let Some(cmd) = &msg.data_string {
            // Execute the command as if it was a bash script.
            let output = Command::new("bash").arg("-c").arg(cmd).output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let resp = format!("{}{}", stdout, stderr);
                    if output.status.success() && stderr.is_empty() {
                        resp
                    } else {
                        format!("Error: {}", resp)
                    }
                }
                Err(e) => {
                    let error_msg = format!(
                        "Error: do_exec({:?}, {:?}) error 1: {}",
                        msg.workdir_idx, cmd, e
                    );
                    log::error!("{}", error_msg);
                    error_msg
                }
            }
        } else {
            let error_msg = format!(
                "Error: do_exec({:?}, None) error 2: No command to execute",
                msg.workdir_idx
            );
            log::error!("{}", error_msg);
            error_msg
        };

        if let Some(resp_channel) = msg.resp_channel {
            if let Err(e) = resp_channel.send(resp) {
                let error_msg = format!(
                    "Error: do_exec({:?}, {:?}) error 3: {}",
                    msg.workdir_idx, msg.data_string, e
                );
                log::error!("{}", error_msg);
            }
        }
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a suibase internal message (not a websocket message!).
            let mut event_rx = self.params.event_rx.lock().await;
            if let Some(msg) = event_rx.recv().await {
                // Process the message.
                drop(event_rx);
                self.do_exec(msg).await;
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }
}
