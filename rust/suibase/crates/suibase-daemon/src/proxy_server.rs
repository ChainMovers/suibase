use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use std::sync::Arc;

use crate::app_error::AppError;
use crate::basic_types::*;

use crate::network_monitor::{
    NetMonTx, NetmonFlags, NetworkMonitor, ProxyHandlerReport, HEADER_SBSD_SERVER_HC,
    HEADER_SBSD_SERVER_IDX,
};
use crate::shared_types::{
    Globals, REQUEST_FAILED_BODY_READ, REQUEST_FAILED_NO_SERVER_AVAILABLE,
    REQUEST_FAILED_NO_SERVER_RESPONDING, REQUEST_FAILED_RESP_BUILDER, REQUEST_FAILED_RESP_BYTES_RX,
    SEND_FAILED_RESP_HTTP_STATUS, SEND_FAILED_UNSPECIFIED_ERROR, SEND_FAILED_UNSPECIFIED_STATUS,
};

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, Request, Response},
    routing::get,
    Router,
};

use hyper::http;
use tokio_graceful_shutdown::SubsystemHandle;
use tower::retry;

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
        report: &mut ProxyHandlerReport,
    ) -> Option<TargetServerIdx> {
        if let Some(server_idx) = headers.remove(HEADER_SBSD_SERVER_IDX) {
            if let Ok(server_idx) = server_idx.to_str().unwrap().parse::<u8>() {
                let stats_flags = report.mut_flags();
                stats_flags.insert(NetmonFlags::HEADER_SBSD_SERVER_IDX_SET);
                return Some(server_idx);
            }
        }
        None
    }

    fn process_header_server_health_check(
        headers: &mut HeaderMap,
        report: &mut ProxyHandlerReport,
    ) -> bool {
        if let Some(_prot_code) = headers.remove(HEADER_SBSD_SERVER_HC) {
            // TODO: validate the prot_code...
            let stats_flags = report.mut_flags();
            stats_flags.insert(NetmonFlags::HEADER_SBSD_SERVER_HC_SET);
            return true;
        }
        false
    }

    async fn proxy_handler(
        State(states): State<Arc<SharedStates>>,
        req: Request<Body>,
    ) -> Result<Response<Body>, AppError> {
        // Statistic Accumulation Design
        //
        // This function *must* call one of the following only once:
        //   (1) report.req_resp_ok
        //          OR
        //   (2) report.req_fail
        //          OR
        //   (3) report.req_resp_err
        //
        // This will properly accumulate the statistics *once* per request.
        //
        // When to use each:
        // (1) req_resp_ok is for when the request and response were both sucessful.
        // (2) req_fail is when the request could not even be sent (after all retries).
        // (3) req_resp_err is for all scenario where a response was received, but an
        //     error was detected in the response.
        //
        // Now, there could be more than one failed send() attempt, and for
        // these the following function can be called multiple times:
        //    - report.send_failed

        let handler_start = EpochTimestamp::now();
        let mut report = ProxyHandlerReport::new(&states.netmon_tx, states.port_idx, handler_start);

        // Identify additional processing just by interpreting headers.
        // At same time:
        //  - Remove custom headers (X-SBSD-) from the request.
        //  - Start building the flags used later for stats/debugging.
        //
        let mut headers = req.headers().clone();

        log::debug!("headers: {:?}", headers);

        //let do_http_body_extraction = ProxyServer::is_json_content_type(&headers);
        let do_force_target_server_idx =
            ProxyServer::process_header_server_idx(&mut headers, &mut report);

        let _ = ProxyServer::process_header_server_health_check(&mut headers, &mut report);
        headers.remove(header::HOST); // Remove the host header (will be replace with the target server).

        // Find which target servers to send to...
        let mut targets: Vec<(u8, String)> = Vec::new();
        {
            let globals_read_guard = states.globals.read().await;
            let globals = &*globals_read_guard;
            if let Some(input_port) = globals.input_ports.get(states.port_idx) {
                if let Some(target_server_idx) = do_force_target_server_idx {
                    if let Some(target_server) = input_port.target_servers.get(target_server_idx) {
                        targets.push((target_server_idx, target_server.uri()));
                    }
                } else {
                    input_port.get_best_target_servers(&mut targets, &handler_start)
                }
            }
        }
        let targets = &targets; // Make immutable.

        let mut retry_count = 0;

        if targets.is_empty() {
            let _perf_report = report
                .req_fail(retry_count, REQUEST_FAILED_NO_SERVER_AVAILABLE)
                .await;
            return Err(anyhow!("No server available").into());
        }

        // Because can have to do potential retry, have to deserialize the body
        // into bytes here (to keep a copy).
        //
        // TODO interpret the JSON to identify what is safe to retry.

        // TODO Optimize (eliminate clone) when there is no retry possible?

        let method = req.method().clone();
        let bytes = hyper::body::to_bytes(req.into_body()).await;

        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(err) => {
                let _perf_report = report.req_fail(retry_count, REQUEST_FAILED_BODY_READ).await;
                return Err(err.into());
            }
        };

        for (server_idx, target_uri) in targets.iter() {
            // Build the request toward the current target server.
            let req_builder = states
                .client
                .request(method.clone(), target_uri)
                .headers(headers.clone())
                .body(bytes.clone());

            // Following works also (if one day bytes and cloning won't be needed):
            //       .body(req.into_body())

            let req_initiation_time = EpochTimestamp::now();
            // Execute the request.
            let resp = req_builder.send().await;

            let resp = match resp {
                Ok(resp) => resp,
                Err(_err) => {
                    // TODO Map _err to SendFailureReason for debugging.

                    // Report a 'send' error, which is a failure to connect to a target server.
                    // This is not intended to count in the total *request* count stats (because
                    // may succeed on a retry on another server) but will affect the health score
                    // of this target server.
                    let _ = report
                        .send_failed(
                            *server_idx,
                            req_initiation_time,
                            SEND_FAILED_UNSPECIFIED_ERROR,
                            http::StatusCode::INTERNAL_SERVER_ERROR,
                        )
                        .await;
                    // Try with another server.
                    retry_count += 1;
                    continue;
                }
            };

            let resp_received = EpochTimestamp::now();

            // Check HTTP errors
            let resp = match resp.error_for_status() {
                Ok(resp) => resp,
                Err(err) => {
                    // Decide if trying another server or not depending if the HTTP
                    // problem is with the request or with the server.
                    // When in doubt, this will assume a problem with the server.
                    //
                    // Note: http_response_err does a "req_fail" when returning false.
                    let try_next_server = report
                        .http_response_error(
                            server_idx,
                            req_initiation_time,
                            resp_received,
                            retry_count,
                            &err,
                        )
                        .await;
                    if try_next_server {
                        retry_count += 1;
                        continue;
                    } else {
                        return Err(err.into());
                    }
                }
            };

            let resp_bytes = resp.bytes().await;

            let resp_bytes = match resp_bytes {
                Ok(resp_bytes) => resp_bytes,
                Err(err) => {
                    let _ = report
                        .req_resp_err(
                            *server_idx,
                            req_initiation_time,
                            resp_received,
                            retry_count,
                            REQUEST_FAILED_RESP_BYTES_RX,
                        )
                        .await;
                    // TODO worth logging a few of these.
                    return Err(err.into());
                }
            };

            let builder = Response::builder().body(Body::from(resp_bytes));

            let resp = match builder {
                Ok(resp) => resp,
                Err(err) => {
                    let _ = report
                        .req_resp_err(
                            *server_idx,
                            req_initiation_time,
                            resp_received,
                            retry_count,
                            REQUEST_FAILED_RESP_BUILDER,
                        )
                        .await;
                    // TODO worth logging a few of these.
                    return Err(err.into());
                }
            };

            // TODO Parse the http::response, detect bad requests and call 'req_resp_err'

            let _ = report
                .req_resp_ok(*server_idx, req_initiation_time, resp_received, retry_count)
                .await;

            return Ok(resp);
        }

        // If we get here, then all the retries failed.
        let _ = report
            .req_fail(retry_count, REQUEST_FAILED_NO_SERVER_RESPONDING)
            .await;

        Err(anyhow!(format!("No server responding ({})", retry_count)).into())
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
