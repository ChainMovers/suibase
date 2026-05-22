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
    REQUEST_FAILED_NO_SERVER_AVAILABLE, REQUEST_FAILED_NO_SERVER_RESPONDING,
    REQUEST_FAILED_NOT_GRPC_CAPABLE, REQUEST_FAILED_RESP_BUILDER, REQUEST_FAILED_RESP_BYTES_RX,
    SEND_FAILED_RESP_HTTP_STATUS, SEND_FAILED_UNSPECIFIED_ERROR,
};

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::State,
    http::{header, Request, Response},
    routing::get,
    Router,
};
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ServerBuilder;
use memchr::memmem;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_graceful_shutdown::SubsystemHandle;
use tower_service::Service as TowerService;

use crate::proxy_grpc::{
    forward_to_upstream, grpc_no_upstream_response, is_grpc_request, BufferedGrpcRequest,
    ForwardOutcome, GrpcProxyClient,
};

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

        // Separate failure types with different limits
        let mut server_failures = 0; // Real server problems (connection, HTTP)
        let mut rate_limit_attempts = 0; // Rate limiting attempts (temporary)

        // Find which target servers to send to...
        let mut targets: Vec<(u8, String)> = Vec::new();
        {
            let globals_read_guard = states.globals.read().await;
            let globals = &*globals_read_guard;

            if let Some(input_port) = globals.input_ports.get(states.port_idx) {
                // Check that the proxy is still enabled/running.
                if !input_port.is_proxy_enabled() {
                    let _perf_report = report
                        .req_fail(server_failures, REQUEST_FAILED_CONFIG_DISABLED)
                        .await;
                    return Err(anyhow!(format!(
                        "{} proxy disabled with suibase.yaml (check proxy_enabled settings)",
                        input_port.workdir_name()
                    ))
                    .into());
                }
                /*
                if !input_port.is_user_request_start() {
                    let _perf_report = report
                        .req_fail(server_failures, REQUEST_FAILED_NOT_STARTED)
                        .await;
                    return Err(anyhow!(format!(
                        "{0} not started (did you forget to do '{0} start'?)",
                        input_port.workdir_name()
                    ))
                    .into());
                }*/

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
                .req_fail(server_failures, REQUEST_FAILED_NO_SERVER_AVAILABLE)
                .await;
            return Err(anyhow!("No server available").into());
        }

        // Because can have to do potential retry, have to deserialize the body
        // into bytes here (to keep a copy).
        //
        // TODO interpret the JSON to identify what is safe to retry.
        //
        // TODO Optimize (eliminate clone) when there is no retry possible?
        let method = req.method().clone();

        // Capture the original headers and client's Accept-Encoding preference
        let original_headers = req.headers().clone();
        let _client_accept_encoding = original_headers.get(header::ACCEPT_ENCODING).cloned();
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
                let _perf_report = report.req_fail(0, REQUEST_FAILED_BODY_READ).await;
                return Err(err.into());
            }
        };

        const MAX_SERVER_FAILURES: u8 = 4; // Must be >= 1, original limit for real failures
        let max_cycles: u8 = (targets.len() as u8).saturating_mul(3); // Allow 3 full cycles
        let mut cycle_count = 0;
        let mut server_index = 0; // Current server index for cycling

        while server_failures < MAX_SERVER_FAILURES && cycle_count < max_cycles {
            // Give a break between cycles to avoid hammering the same server(s) too fast.
            if server_index == 0 {
                if cycle_count > 0 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                cycle_count += 1; //
            }

            let (server_idx, target_uri) = &targets[server_index];
            server_index = (server_index + 1) % targets.len(); // Advance to next server (for next iteration if any).

            // Build the request toward the current target server.
            // Forward all original headers including Accept-Encoding
            let req_builder = states
                .client
                .request(method.clone(), target_uri)
                .headers(original_headers.clone())
                .body(bytes.clone());

            // Following works also (if one day bytes and cloning won't be needed):
            //       .body(req.into_body())

            // Check rate limit before sending the request
            {
                let globals_read_guard = states.globals.read().await;
                let globals = &*globals_read_guard;

                if let Some(input_port) = globals.input_ports.get(states.port_idx) {
                    if let Some(target_server) = input_port.target_servers.get(*server_idx) {
                        if target_server.try_acquire_token().is_err() {
                            // Rate limit exceeded for this server - DON'T count as server failure
                            rate_limit_attempts += 1;
                            let _ = report.rate_limited(*server_idx).await;
                            continue; // try next server
                        }
                    }
                }
            } // Release the read lock

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
                    // Connection failure - count as server failure
                    server_failures += 1;
                    continue; // try next server
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
                            server_failures,
                            &err,
                        )
                        .await;
                    if try_next_server {
                        // HTTP error - count as server failure
                        server_failures += 1;
                        continue; // try next server
                    } else {
                        return Err(err.into());
                    }
                }
            };

            // Capture response headers before consuming the body
            let response_headers = resp.headers().clone();
            let response_status = resp.status();

            // Note: reqwest automatically decompresses gzip/deflate/brotli responses

            let resp_bytes = match resp.bytes().await {
                Ok(resp_bytes) => resp_bytes,
                Err(err) => {
                    let _ = report
                        .req_resp_err(
                            *server_idx,
                            req_initiation_time,
                            resp_received,
                            server_failures,
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
                if let Ok(json_resp) = serde_json::from_slice::<serde_json::Value>(&resp_bytes) {
                    // Check for a failed JSON-RPC that can be safely retried.
                    if let Ok(safe_retry_approved) =
                        Self::is_retryable_sui_level_error(&bytes, &json_resp).await
                    {
                        if safe_retry_approved {
                            // Note: not a server_failure. The server is working
                            // but an object is not found. This is sometimes OK
                            // because the object may exists but was not yet created
                            // for the RPC server. So give a change for the RPC
                            // servers to catch up.
                            tokio::time::sleep(Duration::from_millis(200)).await;
                            continue; // try next server
                        }
                    }

                    // This is the standard way to handle JSON-RPC errors (with "error" object).
                    if let Some(err_obj) = json_resp["error"].as_object() {
                        if !err_obj.contains_key("data") {
                            // Insert our own "data" field.
                            let data =
                                JsonRpcErrorDataObject::new(target_uri.clone(), server_failures);
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

            // Determine the final response body - reqwest has automatically decompressed it
            let final_body_bytes = if let Some(modified_resp_bytes) = modified_resp_bytes {
                modified_resp_bytes
            } else {
                resp_bytes
            };

            let mut builder = Response::builder()
                .status(response_status);

            // Copy headers from original response, but exclude Content-Encoding
            // and Content-Length. Content-Encoding is excluded since reqwest
            // already decompressed the body. Content-Length is excluded
            // because the original value referred to the (possibly
            // compressed) upstream byte count; the actual byte length of the
            // body we're about to write may differ. Hyper's strict response
            // framing closes the connection on any length mismatch, so we
            // let the server re-derive Content-Length from the body itself.
            for (name, value) in response_headers.iter() {
                if name != header::CONTENT_ENCODING && name != header::CONTENT_LENGTH {
                    builder = builder.header(name, value);
                }
            }

            let builder = builder.body(Body::from(final_body_bytes));

            let resp = match builder {
                Ok(resp) => resp,
                Err(err) => {
                    let _ = report
                        .req_resp_err(
                            *server_idx,
                            req_initiation_time,
                            resp_received,
                            server_failures,
                            REQUEST_FAILED_RESP_BUILDER,
                        )
                        .await;
                    // TODO worth logging a few of these.
                    return Err(err.into());
                }
            };

            let _ = report
                .req_resp_ok(
                    *server_idx,
                    req_initiation_time,
                    resp_received,
                    server_failures,
                )
                .await;

            return Ok(resp);
        } // while (server_failures < MAX_SERVER_FAILURES && rate_limit_attempts < max_cycles)

        // If we get here, then all the retries failed.
        let _ = report
            .req_fail(server_failures, REQUEST_FAILED_NO_SERVER_RESPONDING)
            .await;

        Err(anyhow!(format!(
            "No server responding (server_failures: {}, rate_limit_attempts: {})",
            server_failures, rate_limit_attempts
        ))
        .into())
    }

    /// Dispatch an inbound gRPC request to upstream target servers.
    ///
    /// Builds a prioritized list of upstreams:
    ///   1. JSON-RPC-healthy "best" servers first (same selection the JSON-RPC
    ///      handler uses)
    ///   2. Then every remaining target server, regardless of JSON-RPC health
    ///
    /// Upstreams in (2) are still tried because JSON-RPC health doesn't
    /// reflect gRPC capability: a public RPC gateway can be JSON-RPC-healthy
    /// yet not serve gRPC at all, while the official MystenLabs fullnode can
    /// be marked JSON-RPC-degraded but still serve gRPC fine. `forward_unary`
    /// falls through any upstream whose response isn't gRPC-shaped, so
    /// duplicate attempts against non-gRPC servers are cheap.
    ///
    /// TODO: report attempts to NetworkMonitor so `testnet links` counters
    /// update for gRPC traffic too.
    async fn grpc_dispatch(
        req: hyper::Request<hyper::body::Incoming>,
        shared: Arc<SharedStates>,
        grpc_client: Arc<GrpcProxyClient>,
    ) -> hyper::Response<crate::proxy_grpc::ProxyBody> {
        let handler_start = EpochTimestamp::now();
        let mut report = ProxyHandlerReport::new(
            &shared.netmon_tx,
            shared.port_idx,
            handler_start,
        );

        // If the caller specified a single target via the X-SBSD-SERVER-IDX
        // header (e.g. the periodic health-check probe), use only that one
        // upstream and skip the fallback list. Same convention as the
        // existing JSON-RPC handler.
        let forced_idx: Option<u8> = req
            .headers()
            .get(HEADER_SBSD_SERVER_IDX)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u8>().ok());

        // X-SBSD-SERVER-HC marks the request as a controlled health-check
        // probe (vs. user traffic). NetworkMonitor uses this flag to feed
        // `handle_latency_report` so the `RespT ms` column on `<workdir>
        // links` updates from probe samples. Without this, RespT shows "-"
        // even when the server is happily serving gRPC traffic.
        let is_health_check = req.headers().contains_key(HEADER_SBSD_SERVER_HC);
        if is_health_check {
            report
                .mut_flags()
                .insert(NetmonFlags::HEADER_SBSD_SERVER_HC_SET);
        }
        // Tag all gRPC reports so the netmon thread can update the
        // protocol-specific gRPC-capability flag (independent of is_healthy).
        report.mut_flags().insert(NetmonFlags::GRPC_TRAFFIC);

        // Build the prioritized upstream list. Filter on the gRPC-capability
        // flag in ServerStats (independent of JSON-RPC health) so an upstream
        // previously seen returning non-gRPC isn't repeatedly retried for
        // gRPC traffic. A forced index (probe / explicit X-SBSD-SERVER-IDX)
        // bypasses this filter so the probe can re-verify the capability.
        let upstreams: Vec<(u8, String)> = {
            let globals_read = shared.globals.read().await;
            let globals = &*globals_read;
            match globals.input_ports.get(shared.port_idx) {
                Some(input_port) if input_port.is_proxy_enabled() => {
                    if let Some(idx) = forced_idx {
                        match input_port.target_servers.get(idx) {
                            Some(ts) => vec![(idx, ts.rpc())],
                            None => Vec::new(),
                        }
                    } else {
                        let mut best: Vec<(u8, String)> = Vec::new();
                        input_port.get_best_target_servers(&mut best, &handler_start);
                        let mut seen_idx: std::collections::HashSet<u8> =
                            std::collections::HashSet::new();
                        let mut ordered: Vec<(u8, String)> = Vec::new();
                        let grpc_capable = |idx: u8| -> bool {
                            input_port
                                .target_servers
                                .get(idx)
                                .map(|ts| ts.stats.is_grpc_capable())
                                .unwrap_or(true)
                        };
                        for (idx, url) in best {
                            if grpc_capable(idx) && seen_idx.insert(idx) {
                                ordered.push((idx, url));
                            }
                        }
                        for (idx, ts) in input_port.target_servers.iter() {
                            if grpc_capable(idx) && seen_idx.insert(idx) {
                                ordered.push((idx, ts.rpc()));
                            }
                        }
                        ordered
                    }
                }
                _ => Vec::new(),
            }
        };

        // Distinguish "no target servers configured" (NO_SERVER_AVAILABLE)
        // from "proxy explicitly disabled" (CONFIG_DISABLED). The JSON-RPC
        // handler makes this distinction; the gRPC path should too so the
        // operator sees the same error semantics regardless of protocol.
        let proxy_disabled = {
            let globals_read = shared.globals.read().await;
            let globals = &*globals_read;
            !globals
                .input_ports
                .get(shared.port_idx)
                .map(|p| p.is_proxy_enabled())
                .unwrap_or(false)
        };

        if upstreams.is_empty() {
            let (reason, msg) = if proxy_disabled {
                (
                    REQUEST_FAILED_CONFIG_DISABLED,
                    "proxy disabled (check proxy_enabled in suibase.yaml)",
                )
            } else {
                (REQUEST_FAILED_NO_SERVER_AVAILABLE, "no upstream available")
            };
            log::warn!("grpc: {} for port {}", msg, shared.port_idx);
            let _ = report.req_fail(0, reason).await;
            return crate::proxy_grpc::error_proxy_response(
                hyper::StatusCode::SERVICE_UNAVAILABLE,
                msg,
            );
        }

        // Buffer the request body once so we can retry across upstreams.
        let buffered = match BufferedGrpcRequest::from_request(req).await {
            Ok(b) => b,
            Err(early_response) => {
                let _ = report.req_fail(0, REQUEST_FAILED_BODY_READ).await;
                return early_response;
            }
        };

        // Bound the retry fan-out (matches the JSON-RPC handler's
        // MAX_SERVER_FAILURES=4 ceiling). Without this, one failing gRPC
        // request can hold a client up to N × UPSTREAM_TIMEOUT (= N × 30s)
        // when N upstreams are all dead.
        const MAX_SERVER_FAILURES: u8 = 4;

        // Drive the iteration here so we can:
        //  - check the per-server rate limiter (which is also what populates
        //    the QPS/QPM counters shown by `<workdir> links`)
        //  - emit per-attempt stats to NetworkMonitor as we go
        //
        // The per-attempt error reports use `req_resp_err_per_attempt` so
        // port-level all_servers counters only update once (via the final
        // `req_fail` below). This preserves per-target force_down on
        // NOT_GRPC_CAPABLE without inflating port stats by retry-count.
        let mut server_failures: u8 = 0;
        let mut rate_limit_attempts: u8 = 0;
        let mut last_error_response: Option<hyper::Response<crate::proxy_grpc::ProxyBody>> = None;
        let mut last_status_for_msg: Option<hyper::StatusCode> = None;

        for (server_idx, base_url) in upstreams.iter() {
            if server_failures >= MAX_SERVER_FAILURES {
                break;
            }
            // Per-attempt rate-limit check. `try_acquire_token` is what
            // increments the rate_limiter's QPS/QPM counters, so we MUST
            // call it once per attempt even if no limit is configured.
            {
                let globals_read = shared.globals.read().await;
                let globals = &*globals_read;
                if let Some(input_port) = globals.input_ports.get(shared.port_idx) {
                    if let Some(ts) = input_port.target_servers.get(*server_idx) {
                        if ts.try_acquire_token().is_err() {
                            rate_limit_attempts =
                                rate_limit_attempts.saturating_add(1);
                            let _ = report.rate_limited(*server_idx).await;
                            continue;
                        }
                    }
                }
            }

            let req_initiation_time = EpochTimestamp::now();
            let outcome =
                forward_to_upstream(&buffered, base_url, Arc::clone(&grpc_client)).await;
            let resp_received_time = EpochTimestamp::now();
            let retry_count = server_failures;

            match outcome {
                ForwardOutcome::GrpcResponse(resp) => {
                    let _ = report
                        .req_resp_ok(
                            *server_idx,
                            req_initiation_time,
                            resp_received_time,
                            retry_count,
                        )
                        .await;
                    return resp;
                }
                ForwardOutcome::NotGrpc {
                    status,
                    content_type,
                } => {
                    log::warn!(
                        "grpc: upstream '{}' returned non-gRPC response (HTTP {}, content-type='{}'); marking NOT_GRPC_CAPABLE",
                        base_url,
                        status,
                        content_type
                    );
                    let _ = report
                        .req_resp_err_per_attempt(
                            *server_idx,
                            req_initiation_time,
                            resp_received_time,
                            retry_count,
                            REQUEST_FAILED_NOT_GRPC_CAPABLE,
                        )
                        .await;
                    last_status_for_msg = Some(status);
                    server_failures = server_failures.saturating_add(1);
                }
                ForwardOutcome::HttpError {
                    status,
                    content_type,
                } => {
                    // Transient: upstream is reachable and answered, but with
                    // a non-2xx HTTP status (likely 429/5xx). Do NOT mark
                    // NOT_GRPC_CAPABLE — that's reserved for definitive
                    // capability signals (HTTP 200 + non-gRPC body). Report
                    // as a target-only send failure so the score gradually
                    // degrades but doesn't force_down on one bad sample.
                    log::warn!(
                        "grpc: upstream '{}' returned transient HTTP error (HTTP {}, content-type='{}')",
                        base_url,
                        status,
                        content_type
                    );
                    let _ = report
                        .send_failed(
                            *server_idx,
                            req_initiation_time,
                            SEND_FAILED_RESP_HTTP_STATUS,
                            status.as_u16(),
                        )
                        .await;
                    last_status_for_msg = Some(status);
                    server_failures = server_failures.saturating_add(1);
                }
                ForwardOutcome::Error(msg) => {
                    log::warn!("grpc: upstream '{}' send failed: {}", base_url, msg);
                    let _ = report
                        .send_failed(
                            *server_idx,
                            req_initiation_time,
                            SEND_FAILED_UNSPECIFIED_ERROR,
                            0,
                        )
                        .await;
                    last_error_response = Some(grpc_no_upstream_response(&msg));
                    server_failures = server_failures.saturating_add(1);
                }
            }
        }

        // No upstream produced a gRPC response. Issue ONE final all-servers
        // event for this user request.
        let _ = report
            .req_fail(server_failures, REQUEST_FAILED_NO_SERVER_RESPONDING)
            .await;

        if let Some(status) = last_status_for_msg {
            grpc_no_upstream_response(&format!(
                "no gRPC-capable upstream (last upstream returned HTTP {})",
                status
            ))
        } else if let Some(resp) = last_error_response {
            resp
        } else {
            // All attempts were rate-limited.
            grpc_no_upstream_response(&format!(
                "rate-limited on all upstreams ({} attempts)",
                rate_limit_attempts
            ))
        }
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

        let app: Router = Router::new()
            .fallback(get(Self::proxy_handler).post(Self::proxy_handler))
            .with_state(shared_states.clone());

        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port_number);
        log::info!("listening on {}", bind_address);

        // Build the gRPC client once and share it via Arc.
        let grpc_client: Arc<GrpcProxyClient> = match GrpcProxyClient::new() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                log::error!("failed to construct grpc client: {}", e);
                return Err(anyhow!("grpc client init failed: {}", e));
            }
        };

        let listener = TcpListener::bind(&bind_address)
            .await
            .map_err(|e| anyhow!("bind {} failed: {}", bind_address, e))?;

        // Accept loop. For each connection we let hyper-util's `auto::Builder`
        // negotiate HTTP/1.1 vs HTTP/2 from the client's preface. Inside the
        // per-request `service_fn` we dispatch by content-type:
        //   - `application/grpc*`  -> gRPC unary forwarder (preserves trailers)
        //   - anything else        -> existing axum-based JSON-RPC handler
        let app_for_loop = app.clone();
        let grpc_client_for_loop = grpc_client.clone();
        let shared_for_loop = shared_states.clone();

        // Per-connection tasks are tracked in a JoinSet so we can drain them
        // on shutdown. Without this, in-flight requests would be killed when
        // the runtime tears down (TCP RST mid-response). Matches the prior
        // axum graceful_shutdown(60s) behavior. The 60s ceiling bounds how
        // long shutdown blocks waiting for stragglers.
        const SHUTDOWN_DRAIN: Duration = Duration::from_secs(60);
        let mut connection_tasks: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();

        let accept_loop = async {
            loop {
                let (stream, _peer) = match listener.accept().await {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("accept error on {}: {}", bind_address, e);
                        continue;
                    }
                };
                let io = TokioIo::new(stream);
                let app_clone = app_for_loop.clone();
                let grpc_client_conn = grpc_client_for_loop.clone();
                let shared_conn = shared_for_loop.clone();

                connection_tasks.spawn(async move {
                    let svc = hyper::service::service_fn(
                        move |req: hyper::Request<hyper::body::Incoming>| {
                            let mut app = app_clone.clone();
                            let grpc_client = grpc_client_conn.clone();
                            let shared = shared_conn.clone();
                            async move {
                                let response: hyper::Response<crate::proxy_grpc::ProxyBody> =
                                    if is_grpc_request(&req) {
                                        Self::grpc_dispatch(req, shared, grpc_client).await
                                    } else {
                                        // Hand off to the existing axum router.
                                        // axum's Router is a `tower::Service`; we
                                        // convert hyper's request body into
                                        // `axum::body::Body` and back.
                                        let (parts, body) = req.into_parts();
                                        let axum_req = axum::http::Request::from_parts(
                                            parts,
                                            axum::body::Body::new(body),
                                        );
                                        let resp = match app.call(axum_req).await {
                                            Ok(r) => r,
                                            Err(infallible) => match infallible {},
                                        };
                                        let (parts, body) = resp.into_parts();
                                        // Re-box axum's body into our shared
                                        // error type so both arms return the
                                        // same `Response<ProxyBody>`.
                                        let boxed: crate::proxy_grpc::ProxyBody = body
                                            .map_err(|e| -> crate::proxy_grpc::ProxyError {
                                                Box::new(e)
                                            })
                                            .boxed_unsync();
                                        hyper::Response::from_parts(parts, boxed)
                                    };
                                Ok::<_, std::convert::Infallible>(response)
                            }
                        },
                    );

                    let builder = ServerBuilder::new(TokioExecutor::new());
                    if let Err(e) = builder.serve_connection(io, svc).await {
                        log::debug!("connection closed with error: {}", e);
                    }
                });
            }
        };

        // Race the accept loop against subsystem shutdown. On shutdown we
        // stop accepting new connections and then drain in-flight ones.
        tokio::select! {
            _ = accept_loop => {},
            _ = subsys.on_shutdown_requested() => {
                log::info!("shutdown requested for {}; draining in-flight requests", bind_address);
            }
        }

        // Drain phase: wait up to SHUTDOWN_DRAIN for live request tasks to
        // finish. Any task still running after the deadline is aborted so
        // shutdown completes in bounded time.
        let drain = async {
            while connection_tasks.join_next().await.is_some() {}
        };
        match tokio::time::timeout(SHUTDOWN_DRAIN, drain).await {
            Ok(()) => log::info!("drained all connections for {}", bind_address),
            Err(_) => {
                log::warn!(
                    "drain timeout ({}s) elapsed for {} with {} live connections; aborting",
                    SHUTDOWN_DRAIN.as_secs(),
                    bind_address,
                    connection_tasks.len()
                );
                connection_tasks.shutdown().await;
            }
        }

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
                if sui_req_method.as_str() == "sui_getEvents"
                    && (message.contains("not find") || message.contains("otExists"))
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
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
