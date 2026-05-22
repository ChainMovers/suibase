use crate::network_monitor::NetmonMsg;

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use crate::network_monitor::{NetMonRx, HEADER_SBSD_SERVER_HC, HEADER_SBSD_SERVER_IDX};

/// gRPC unary path used for periodic capability/health probing.
///
/// `GetServiceInfo` is a no-parameter unary call on `LedgerService`; the
/// response contains chain id, epoch, and current checkpoint height — small
/// and cheap. The proxy's content-type sniffer routes this request to the
/// gRPC forwarder, which marks the targeted upstream as NOT_GRPC_CAPABLE
/// (and force_down) if it answers with HTML/JSON instead of gRPC.
const HEALTH_CHECK_GRPC_PATH: &str = "/sui.rpc.v2.LedgerService/GetServiceInfo";

/// gRPC wire frame for an empty request message:
///   byte 0   : compression flag (0 = uncompressed)
///   bytes 1-4: message length, big-endian (0)
/// followed by 0 bytes of payload.
const EMPTY_GRPC_FRAME: [u8; 5] = [0, 0, 0, 0, 0];

pub struct RequestWorker {
    netmon_rx: NetMonRx,
    client: reqwest::Client,
}

impl RequestWorker {
    pub fn new(netmon_rx: NetMonRx) -> Self {
        // `http2_prior_knowledge` skips the HTTP/1.1 upgrade dance — required
        // since gRPC is HTTP/2-only and the proxy listener is local h2c.
        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .build()
            .expect("reqwest client builder");
        Self { netmon_rx, client }
    }

    async fn do_request(&mut self, msg: NetmonMsg) {
        let server_idx = msg.server_idx().to_string();

        // Hit the local proxy's own listener; the proxy then forwards to the
        // specific upstream named by X-SBSD-SERVER-IDX.
        let uri = format!(
            "http://localhost:{}{}",
            msg.para16()[0],
            HEALTH_CHECK_GRPC_PATH
        );
        let _ = self
            .client
            .request(reqwest::Method::POST, uri)
            .timeout(std::time::Duration::from_secs(5))
            .header(reqwest::header::CONTENT_TYPE, "application/grpc")
            .header("te", "trailers")
            .header(HEADER_SBSD_SERVER_IDX, server_idx.as_str())
            .header(HEADER_SBSD_SERVER_HC, "1")
            .body(EMPTY_GRPC_FRAME.to_vec())
            .send()
            .await;

        // No return value: the proxy's gRPC dispatch reports ok/err to
        // NetworkMonitor on our behalf as part of normal traffic accounting.
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = self.netmon_rx.recv().await {
                common::mpsc_q_check!(self.netmon_rx);
                // Process the message.
                self.do_request(msg).await;
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

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
