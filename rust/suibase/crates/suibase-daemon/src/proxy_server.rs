use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use std::sync::Arc;

use crate::app_error::AppError;
use crate::basic_types::*;
use crate::globals::Globals;
use crate::network_monitor::{
    NetMonTx, NetmonFlags, NetworkMonitor, HEADER_SBSD_SERVER_HC, HEADER_SBSD_SERVER_IDX,
};

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, Request, Response},
    routing::get,
    Router,
};

use tokio_graceful_shutdown::SubsystemHandle;

// An application target the localhost:port
//
// Each workdir should have a unique port assigned.
//
// The HashMap key is the port number.
//
#[derive(Clone)]
pub struct SharedStates {
    port_idx: ManagedVecUSize,
    client: reqwest::Client,
    netmon_tx: NetMonTx,
    globals: Globals,
}

pub struct ProxyServer {
    enabled: bool,
}

impl ProxyServer {
    pub fn new() -> Self {
        Self { enabled: false }
    }

    // From https://docs.rs/axum/0.6.18/src/axum/json.rs.html#147
    fn is_json_content_type(headers: &HeaderMap) -> bool {
        let content_type = if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
            content_type
        } else {
            return false;
        };

        let content_type = if let Ok(content_type) = content_type.to_str() {
            content_type
        } else {
            return false;
        };

        let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
            mime
        } else {
            return false;
        };

        let is_json_content_type = mime.type_() == "application"
            && (mime.subtype() == "json" || mime.suffix().map_or(false, |name| name == "json"));

        is_json_content_type
    }

    fn process_header_server_idx(
        headers: &mut HeaderMap,
        stats_flags: &mut NetmonFlags,
    ) -> Option<TargetServerIdx> {
        if let Some(server_idx) = headers.remove(HEADER_SBSD_SERVER_IDX) {
            if let Ok(server_idx) = server_idx.to_str().unwrap().parse::<u8>() {
                stats_flags.insert(NetmonFlags::HEADER_SBSD_SERVER_IDX_SET);
                return Some(server_idx);
            }
        }
        None
    }

    fn process_header_server_health_check(
        headers: &mut HeaderMap,
        stats_flags: &mut NetmonFlags,
    ) -> bool {
        if let Some(_prot_code) = headers.remove(HEADER_SBSD_SERVER_HC) {
            // TODO: validate the prot_code...
            stats_flags.insert(NetmonFlags::HEADER_SBSD_SERVER_HC_SET);
            return true;
        }
        false
    }

    async fn proxy_handler(
        State(states): State<Arc<SharedStates>>,
        req: Request<Body>,
    ) -> Result<Response<Body>, AppError> {
        let handler_start = EpochTimestamp::now();

        // Identify additional processing just by interpreting headers.
        // At same time:
        //  - Remove custom headers (X-SBSD-) from the request.
        //  - Start building the flags used later for stats/debugging.
        //
        let mut headers = req.headers().clone();
        let mut stats_flags: NetmonFlags = NetmonFlags::empty();

        let do_http_body_extraction = ProxyServer::is_json_content_type(&headers);
        let do_force_target_server_idx =
            ProxyServer::process_header_server_idx(&mut headers, &mut stats_flags);

        let _ = ProxyServer::process_header_server_health_check(&mut headers, &mut stats_flags);

        // Find which target server to send to...
        let best_target_found = {
            let globals_read_guard = states.globals.read().await;
            let globals = &*globals_read_guard;

            if let Some(input_port) = globals.input_ports.get(states.port_idx) {
                if let Some(target_server_idx) = do_force_target_server_idx {
                    if let Some(target_server) = input_port.target_servers.get(target_server_idx) {
                        Some((target_server_idx, target_server.uri()))
                    } else {
                        None
                    }
                } else {
                    input_port.find_best_target_server()
                }
            } else {
                None
            }
        };
        if best_target_found.is_none() {
            return Err(anyhow!("No server reacheable").into());
        }
        // deconstruct best_target_found
        let (target_idx, target_uri) = best_target_found.unwrap();

        let method = req.method().clone();

        let req_builder = {
            if do_http_body_extraction {
                let bytes = hyper::body::to_bytes(req.into_body()).await?;

                // TODO: Later we can interpret bytes for more advanced feature!!!

                states
                    .client
                    .request(method, target_uri)
                    .headers(headers)
                    .body(bytes)
            } else {
                states
                    .client
                    .request(method, target_uri)
                    .headers(headers)
                    .body(req.into_body())
            }
        };

        // Build the request toward the best target server.

        let req_initiation_time = EpochTimestamp::now();

        // Execute the request.
        let resp = req_builder.send().await?;

        // Handle the http::response
        let builder = Response::builder().body(Body::from(resp.bytes().await?));
        let resp = builder.unwrap();

        let resp_received = EpochTimestamp::now();

        // Log performance stats (ignore error).
        let _perf_report = NetworkMonitor::report_proxy_handler_resp_ok(
            &states.netmon_tx,
            &mut stats_flags,
            states.port_idx,
            target_idx,
            handler_start,
            req_initiation_time,
            resp_received,
        )
        .await;

        Ok(resp)
    }

    pub async fn run(
        self,
        subsys: SubsystemHandle,
        port_idx: InputPortIdx,
        globals: Globals,
        netmon_tx: NetMonTx,
    ) -> Result<()> {
        let shared_states: Arc<SharedStates> = Arc::new(SharedStates {
            port_idx,
            client: reqwest::Client::builder()
                .no_proxy()
                .connection_verbose(true)
                .build()?,
            globals,
            netmon_tx,
        });

        // Validate access to the PortStates in the Globals with an async confirmation that
        // there is a ProxyServer running for it (which will get clear on any failure to
        // start or later on any reason for thread exit).
        let port_number = {
            // Yes... it is amazingly complicated just to get access... but this is happening rarely
            // and is the price to pay to make "flexible and safe" multi-threaded globals in Rust.
            let mut globals_write_guard = shared_states.globals.write().await;
            let globals = &mut *globals_write_guard;
            let input_ports = &mut globals.input_ports;
            if let Some(input_port) = input_ports.get_mut(port_idx) {
                input_port.report_proxy_server_starting();
                input_port.port_number()
            } else {
                log::error!("port {} not found", port_idx);
                return Err(anyhow!("port {} not found", port_idx));
            }
        };

        let app = Router::new()
            .fallback(get(Self::proxy_handler).post(Self::proxy_handler))
            .with_state(shared_states.clone());

        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port_number);
        log::info!("listening on {}", bind_address);

        let return_value = axum::Server::bind(&bind_address)
            .serve(app.into_make_service())
            .with_graceful_shutdown(subsys.on_shutdown_requested())
            .await
            .map_err(|err| anyhow! {err});

        log::info!("stopped for {}", bind_address);

        {
            // This will cover for all scenario (abnormal or not) that the proxy had to exit. Will
            // allow the AdminController to detect and react as needed.
            let mut globals_write_guard = shared_states.globals.write().await;
            let globals = &mut *globals_write_guard;
            let input_port = &mut globals.input_ports;
            if let Some(port_state) = input_port.get_mut(port_idx) {
                port_state.report_proxy_server_not_running();
            }
        }

        return_value
    }
}
