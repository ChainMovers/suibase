use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use crate::app_error::AppError;
use crate::walrus_monitor::{WalrusMonTx, WalrusStatsReporter};
use common::basic_types::*;

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::State,
    http::{header, Request, Response},
    routing::any,
    Router,
};
use http_body_util::BodyExt;
use tokio_graceful_shutdown::SubsystemHandle;

#[derive(Clone)]
pub struct WalrusRelaySharedStates {
    workdir_idx: WorkdirIdx,
    client: reqwest::Client,
    walrus_tx: WalrusMonTx,
    local_port: u16, // Port of the actual walrus-upload-relay backend
}

pub struct WalrusRelayProxyServer {}

impl WalrusRelayProxyServer {
    pub fn new() -> Self {
        Self {}
    }

    async fn proxy_handler(
        State(state): State<Arc<WalrusRelaySharedStates>>,
        req: Request<Body>,
    ) -> Result<Response<Body>, AppError> {
        // Create stats reporter following ProxyHandlerReport pattern
        let stats_reporter = WalrusStatsReporter::new(&state.walrus_tx, state.workdir_idx);
        
        let method = req.method().clone();
        let uri = req.uri();
        let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

        // Build target URL to local walrus-upload-relay backend
        let target_url = format!("http://localhost:{}{}", state.local_port, path_and_query);

        // Extract headers and body
        let headers = req.headers().clone();
        let body_bytes = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                let error_msg = format!("Failed to read request body: {}", err);
                let _ = stats_reporter.report_failure(error_msg.clone()).await;
                return Err(anyhow!(error_msg).into());
            }
        };

        // Forward request to walrus-upload-relay backend
        let req_builder = state
            .client
            .request(method, &target_url)
            .headers(headers)
            .body(body_bytes);

        let response = match req_builder.send().await {
            Ok(resp) => resp,
            Err(err) => {
                let error_msg = format!("Failed to connect to walrus relay backend: {}", err);
                let _ = stats_reporter.report_failure(error_msg.clone()).await;
                return Err(anyhow!(error_msg).into());
            }
        };

        // Check for HTTP errors
        let response = match response.error_for_status() {
            Ok(resp) => resp,
            Err(err) => {
                let error_msg = format!("HTTP error from walrus relay backend: {}", err);
                let _ = stats_reporter.report_failure(error_msg.clone()).await;
                return Err(anyhow!(error_msg).into());
            }
        };

        // Extract response details
        let status = response.status();
        let headers = response.headers().clone();

        // Read response body
        let response_bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                let error_msg = format!("Failed to read response body: {}", err);
                let _ = stats_reporter.report_failure(error_msg.clone()).await;
                return Err(anyhow!(error_msg).into());
            }
        };

        // Build response
        let mut resp_builder = Response::builder().status(status);
        
        // Copy headers from original response, but exclude Content-Encoding 
        // since reqwest has already decompressed the body
        if let Some(headers_map) = resp_builder.headers_mut() {
            for (name, value) in headers.iter() {
                if name != header::CONTENT_ENCODING {
                    headers_map.insert(name, value.clone());
                }
            }
        }

        let response = match resp_builder.body(Body::from(response_bytes)) {
            Ok(resp) => resp,
            Err(err) => {
                let error_msg = format!("Failed to build response: {}", err);
                let _ = stats_reporter.report_failure(error_msg.clone()).await;
                return Err(anyhow!(error_msg).into());
            }
        };

        // Report success
        let _ = stats_reporter.report_success().await;

        Ok(response)
    }

    pub async fn run(
        self,
        subsys: SubsystemHandle,
        workdir_idx: WorkdirIdx,
        proxy_port: u16,
        local_port: u16,
        walrus_tx: WalrusMonTx,
    ) -> Result<()> {
        log::info!(
            "Starting walrus relay proxy server for workdir {} on port {} -> localhost:{}",
            workdir_idx,
            proxy_port,
            local_port
        );

        let shared_states = Arc::new(WalrusRelaySharedStates {
            workdir_idx,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .no_proxy()
                .build()?,
            walrus_tx,
            local_port,
        });

        // Create Axum router that forwards all requests
        let app = Router::new()
            .fallback(any(Self::proxy_handler))
            .with_state(shared_states);

        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), proxy_port);
        log::info!("Walrus relay proxy listening on {}", bind_address);

        let handle = axum_server::Handle::new();

        // Spawn graceful shutdown task
        tokio::spawn(graceful_shutdown(subsys, handle.clone()));

        // Start the server
        axum_server::bind(bind_address)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .unwrap();

        log::info!("Walrus relay proxy stopped for {}", bind_address);
        Ok(())
    }
}

async fn graceful_shutdown(subsys: SubsystemHandle, axum_handle: axum_server::Handle) {
    // Wait for shutdown signal
    subsys.on_shutdown_requested().await;
    // Signal the axum server to shutdown gracefully
    axum_handle.graceful_shutdown(Some(Duration::from_secs(60)));
}