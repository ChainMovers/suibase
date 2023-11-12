// Child thread of events_writer_worker
//
// Responsible to:
//   - websocket auto-reconnect for a single server.
//   - keep alive the connection with Ping
//   - subscribe/unsubscribe to Sui events, filter and forward the
//     validated data to its parent thread.
//
// The thread is auto-restart in case of panic.

use std::{collections::HashSet, sync::Arc};

use crate::{
    admin_controller::AdminControllerRx,
    basic_types::{AutoThread, Runnable, WorkdirIdx},
    shared_types::Globals,
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

#[derive(Clone)]
pub struct WebSocketWorkerParams {
    _globals: Globals,
    event_rx: Arc<Mutex<AdminControllerRx>>,
    workdir_idx: Option<WorkdirIdx>,
}

impl WebSocketWorkerParams {
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

struct WebSocketWorkerThread {
    name: String,
    params: WebSocketWorkerParams,
    // Set of unique packaged id (string).
    subscribed_ids: HashSet<String>,

    // Active websocket connection.
    write: Option<SplitSink<WebSocketStream<TcpStream>, Message>>,
    read: Option<SplitStream<WebSocketStream<TcpStream>>>,

    // Last known valid sequence number processed.
    last_seq_number: u64,
}

#[async_trait]
impl Runnable<WebSocketWorkerParams> for WebSocketWorkerThread {
    fn new(name: String, params: WebSocketWorkerParams) -> Self {
        Self {
            name,
            params,
            subscribed_ids: HashSet::new(),
            write: None,
            read: None,
            last_seq_number: 0,
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
    fn subscribe_request_format(&mut self, id: u64, package: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","method":"suix_subscribeEvent","id":{},"params":[{{"Package":"{}"}}]}}"#,
            id, package
        )
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // TODO - auto-reconnect logic.

        // Open a websocket connection to the server for this workdir.
        let socket_url = "ws://0.0.0.0:9000";

        match connect_async(socket_url).await {
            Ok((ws_stream, _response)) => {
                //self.ws_stream = Some(ws_stream);
                let (write, read) = ws_stream.split();
                self.write = Some(write);
                self.read = Some(read);
            }
            Err(e) => {
                log::error!("connect_async error: {:?}", e);
                // Delay of 5 seconds before retrying.
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                return;
            }
        }

        let msg = Message::Text(self.subscribe_request_format(
            1,
            "9402306344411345bd37243cdbb8743fa7cfcf7c33933feae74f7b2621b57d60",
        ));

        if let Some(ref mut write) = self.write {
            if let Err(e) = write.send(msg).await {
                log::error!("write.send error: {:?}", e);
            } else {
                log::info!("write.send success");
            }
        }

        // Take ownership of the event_rx channel as long this thread is running.
        let mut event_rx = self.params.event_rx.lock().await;

        while !subsys.is_shutdown_requested() {
            let ws_stream_future = futures::FutureExt::fuse(self.read.as_mut().unwrap().next());
            let event_rx_future = futures::FutureExt::fuse(event_rx.recv());

            tokio::select! {
                msg = ws_stream_future => {
                    if let Some(msg) = msg {
                        let msg = msg.unwrap();
                        log::info!("Received a websocket message: {:?}", msg);
                    } else {
                        // Shutdown requested.
                        log::info!("Received a None websocket message");
                        return;
                    }
                }
                msg = event_rx_future => {
                    if let Some(msg) = msg {
                        // Process the message.
                        // drop(event_rx);
                        log::info!("Received an internal message: {:?}", msg);
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
