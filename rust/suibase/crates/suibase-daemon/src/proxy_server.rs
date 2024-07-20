use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use std::sync::Arc;
use std::time::Duration;

use crate::app_error::AppError;

use common::basic_types::*;

use crate::network_monitor::{
    NetMonTx, NetmonFlags, ProxyHandlerReport, HEADER_SBSD_SERVER_HC, HEADER_SBSD_SERVER_IDX,
};
use crate::shared_types::{
    GlobalsProxyMT, REQUEST_FAILED_BODY_READ, REQUEST_FAILED_CONFIG_DISABLED,
    REQUEST_FAILED_NOT_STARTED, REQUEST_FAILED_NO_SERVER_AVAILABLE,
    REQUEST_FAILED_NO_SERVER_RESPONDING, REQUEST_FAILED_RESP_BUILDER, REQUEST_FAILED_RESP_BYTES_RX,
    SEND_FAILED_UNSPECIFIED_ERROR,
};

use anyhow::{anyhow, Result};
use axum::{
    body::{Body, HttpBody},
    extract::State,
    http::{header, Request, Response},
    routing::get,
    Router,
};

use hyper::body::Bytes;
use memchr::memmem;
use serde::{Deserialize, Serialize};
use tokio_graceful_shutdown::SubsystemHandle;

// An application target the localhost:port
//
// Each workdir should have a unique port assigned.
//
// The HashMap key is the port number.
//
#[derive(Clone)]
pub struct SharedStates {
    port_idx: ManagedVecU8,
    client: reqwest::Client,
    netmon_tx: NetMonTx,
    globals: GlobalsProxyMT,
}

pub struct ProxyServer {}

impl ProxyServer {
    pub fn new() -> Self {
        Self {}
    }

    /*
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

        // TODO https://www.jsonrpc.org/historical/json-rpc-over-http.html Check for json-rpc and json request?
        let is_json_content_type = mime.type_() == "application"
            && (mime.subtype() == "json" || mime.suffix().map_or(false, |name| name == "json"));

        is_json_content_type
    }*/

    fn process_header_server_idx(
        headers: &mut axum::http::HeaderMap,
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
        headers: &mut axum::http::HeaderMap,
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
        // (1) req_resp_ok is for when the request and response were both successful.
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

        // log::debug!("headers: {:?}", headers);

        //let is_request_json = ProxyServer::is_json_content_type(&headers);
        let do_force_target_server_idx =
            ProxyServer::process_header_server_idx(&mut headers, &mut report);

        let _ = ProxyServer::process_header_server_health_check(&mut headers, &mut report);
        headers.remove(header::HOST); // Remove the host header (will be replace with the target server).

        let mut retry_count = 0;

        // Find which target servers to send to...
        let mut targets: Vec<(u8, String)> = Vec::new();
        {
            let globals_read_guard = states.globals.read().await;
            let globals = &*globals_read_guard;

            if let Some(input_port) = globals.input_ports.get(states.port_idx) {
                // Check that the proxy is still enabled/running.
                if !input_port.is_proxy_enabled() {
                    let _perf_report = report
                        .req_fail(retry_count, REQUEST_FAILED_CONFIG_DISABLED)
                        .await;
                    return Err(anyhow!(format!(
                        "{} proxy disabled with suibase.yaml (check proxy_enabled settings)",
                        input_port.workdir_name()
                    ))
                    .into());
                }
                if !input_port.is_user_request_start() {
                    let _perf_report = report
                        .req_fail(retry_count, REQUEST_FAILED_NOT_STARTED)
                        .await;
                    return Err(anyhow!(format!(
                        "{0} not started (did you forget to do '{0} start'?)",
                        input_port.workdir_name()
                    ))
                    .into());
                }

                if let Some(target_server_idx) = do_force_target_server_idx {
                    if let Some(target_server) = input_port.target_servers.get(target_server_idx) {
                        targets.push((target_server_idx, target_server.rpc()));
                    }
                } else {
                    input_port.get_best_target_servers(&mut targets, &handler_start)
                }
            }
        }
        let targets = &targets; // Make immutable.

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
        /* This code on hold until deciding to move to hyper v1.0, which is a dependency of reqwest >= 0.11
         * Last time I tried, it just "does not work"... most servers respond with 400-level errors.
        let reqwest_method: reqwest::Method = method.as_str().parse().unwrap();

        // Iterate req.headers().clone() and create an equivalent reqwest::header::HeaderMap.
        // This is needed because reqwest::Client::header() does not accept a hyper::HeaderMap.
        let headers = req.headers();
        let mut reqwest_headers = reqwest::header::HeaderMap::new();
        for (name, value) in headers.iter() {
            let name = reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes()).unwrap();
            let value = reqwest::header::HeaderValue::from_bytes(value.as_bytes()).unwrap();
            reqwest_headers.insert(name, value);
        }*/

        let bytes = match req.into_body().collect().await {
            Ok(body) => body.to_bytes(),
            Err(err) => {
                let _perf_report = report.req_fail(retry_count, REQUEST_FAILED_BODY_READ).await;
                return Err(err.into());
            }
        };

        const MAX_RETRIES: u8 = 4; // Must be >= 1

        for (server_idx, target_uri) in targets.iter() {
            let mut same_server_attempt = true;

            while same_server_attempt && retry_count < MAX_RETRIES {
                same_server_attempt = false; // Will change to true in this loop if need to retry *same* server.

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
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
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

                // TODO Parse the http::response, detect bad requests and call 'req_resp_err'

                // if the response is a JSON error then add proxy specific 'data' to it to help
                // find the problem.
                //
                // Do first a "weak" but "very fast" check before starting to do costly JSON serde.
                //
                // Also, check to retry with a different server some failed requests when safe to do so.

                let mut modified_resp_bytes: Option<Bytes> = None;
                let mut find_json_error = memmem::find_iter(&resp_bytes, "\"error\":");
                if find_json_error.next().is_some() {
                    if let Ok(json_resp) = serde_json::from_slice::<serde_json::Value>(&resp_bytes)
                    {
                        // Check for a failed JSON-RPC that can be safely retried.
                        // Why MAX_RETRIES-1?
                        // At some point, have to stop retrying and return a "success with NotExists error" to
                        // the user (instead of keep going until reaching "failure for too much retry").
                        if retry_count < (MAX_RETRIES - 1) {
                            if let Ok(safe_retry_approved) =
                                Self::is_retryable_sui_level_error(&bytes, &json_resp).await
                            {
                                if safe_retry_approved {
                                    // Safe to retry after a delay of 1 secs.
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                    // Retry with a different server, except when there is no other server
                                    // left to try.
                                    retry_count += 1;
                                    if retry_count as usize >= targets.len() {
                                        same_server_attempt = true;
                                    }
                                    continue;
                                }
                            }
                        }

                        // This is the standard way to handle JSON-RPC errors (with "error" object).
                        if let Some(err_obj) = json_resp["error"].as_object() {
                            if !err_obj.contains_key("data") {
                                // Insert our own "data" field.
                                let data =
                                    JsonRpcErrorDataObject::new(target_uri.clone(), retry_count);
                                let mut json_resp = json_resp.clone();
                                if let Ok(data_obj) = serde_json::to_value(data) {
                                    json_resp["data"] = data_obj;
                                    modified_resp_bytes =
                                        Some(serde_json::to_vec(&json_resp).unwrap().into());
                                }
                            }
                        }
                    }
                }

                let builder = if let Some(modified_resp_bytes) = modified_resp_bytes {
                    Response::builder().body(Body::from(modified_resp_bytes))
                } else {
                    Response::builder().body(Body::from(resp_bytes))
                };

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

                let _ = report
                    .req_resp_ok(*server_idx, req_initiation_time, resp_received, retry_count)
                    .await;

                return Ok(resp);
            } // while (same_server_attempt)
        } // for (server_idx, target_uri)

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
        globals: GlobalsProxyMT,
        netmon_tx: NetMonTx,
    ) -> Result<()> {
        let shared_states: Arc<SharedStates> = Arc::new(SharedStates {
            port_idx,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
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

        let handle = axum_server::Handle::new();

        // Spawn a task to shutdown axum server (on process exit or signal).
        tokio::spawn(graceful_shutdown(subsys, handle.clone()));

        //let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
        axum_server::bind(bind_address)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .unwrap();

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

        Ok(())
    }

    async fn is_retryable_sui_level_error(
        request: &Bytes,
        json_resp: &serde_json::Value,
    ) -> Result<bool> {
        // Extract the JSON-RPC method field from the request.
        let sui_req_method: String =
            if let Ok(json_req) = serde_json::from_slice::<serde_json::Value>(request) {
                if let Some(method) = json_req.get("method").and_then(|v| v.as_str()) {
                    method.to_owned()
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            };
        //log::info!("method: {}", sui_req_method);

        // Extract the result->error->code field from the response.
        //
        // Note: Sui RPC server error format sometimes a "result" object
        // even when it is an error (this is not typical of JSON-RPC).
        if let Some(result_obj) = json_resp.get("result").and_then(|v| v.as_object()) {
            // Handle errors returned within a "suc"
            if let Some(err_obj) = result_obj.get("error").and_then(|v| v.as_object()) {
                if let Some(code_str) = err_obj["code"].as_str() {
                    if code_str == "notExists" {
                        match sui_req_method.as_str() {
                            "suix_getDynamicFieldObject"
                            | "suix_getDynamicFields"
                            | "suix_getOwnedObjects"
                            | "sui_getObject"
                            | "sui_tryGetPastObject" => return Ok(true),
                            _ => (),
                        }
                    }
                }
            }
        } else if let Some(err_obj) = json_resp.get("error").and_then(|v| v.as_object()) {
            if let Some(message) = err_obj.get("message").and_then(|v| v.as_str()) {
                // Example of error:
                // ~$ curl -H "Content-Type: application/json"
                //    -H 'client-target-api-version: 1.28.0' -H 'client-sdk-version: 1.28.0'
                //    --data '{ "id":2, "jsonrpc":"2.0", "method":"sui_getEvents",
                //              "params": ["4UM3m1Kz7p596UVnyr2QNVAMobrfEZV9RYXkMUX8NYxJ"]}' http://localhost:44343
                //
                // Response:
                // {"jsonrpc":"2.0","error":{"code":-32602,
                //   "message":"Could not find the referenced transaction [TransactionDigest(4UM3m1Kz7p596UVnyr2QNVAMobrfEZV9RYXkMUX8NYxJ)]."},
                //  "id":2,"data":{"origin":"https://rpc-mainnet.suiscan.xyz:443","retry":3}}
                match sui_req_method.as_str() {
                    "sui_getEvents" => {
                        if message.contains("not find") || message.contains("otExists") {
                            return Ok(true);
                        }
                    }
                    _ => (),
                }
            }
        }

        Ok(false)
    }
}

async fn graceful_shutdown(subsys: SubsystemHandle, axum_handle: axum_server::Handle) {
    // Run as a thread. Block until shutdown requested.
    subsys.on_shutdown_requested().await;
    // Signal the axum server to shutdown.
    axum_handle.graceful_shutdown(Some(Duration::from_secs(30)));
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct JsonRpcErrorDataObject {
    origin: String,
    retry: u8,
}

impl JsonRpcErrorDataObject {
    fn new(origin: String, retry: u8) -> Self {
        Self { origin, retry }
    }
}
