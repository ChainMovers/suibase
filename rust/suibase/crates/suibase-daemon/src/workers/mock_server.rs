// Mock server worker for testing suibase-daemon proxy server functionality.
//
// Implements HTTP servers that simulate RPC server behaviors including:
// - Configurable failure rates
// - Artificial latency
// - Custom response bodies
// - Statistics tracking

use crate::shared_types::{MockServerState, MockErrorType};

use anyhow::Result;
use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::any,
    Router,
};
use axum::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use common::basic_types::{AutoThread, Runnable};

#[derive(Clone)]
pub struct MockServerParams {
    pub state: Arc<MockServerState>,
}

impl MockServerParams {
    pub fn new(state: Arc<MockServerState>) -> Self {
        Self { state }
    }
}

pub struct MockServerWorker {
    auto_thread: AutoThread<MockServerTask, MockServerParams>,
}

impl MockServerWorker {
    pub fn new(params: MockServerParams) -> Self {
        let name = format!("MockServer({})", params.state.alias);
        Self {
            auto_thread: AutoThread::new(name, params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct MockServerTask {
    task_name: String,
    params: MockServerParams,
}

#[async_trait]
impl Runnable<MockServerParams> for MockServerTask {
    fn new(task_name: String, params: MockServerParams) -> Self {
        Self { task_name, params }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        let output = format!("started {} on port {}", self.task_name, self.params.state.port);
        log::info!("{}", output);

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(_) => {
                log::info!("{} normal task exit (2)", self.task_name);
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("{} normal task exit (1)", self.task_name);
                Ok(())
            }
        }
    }
}

impl MockServerTask {
    async fn event_loop(&mut self, _subsys: &SubsystemHandle) -> Result<()> {
        // Single route that handles every method/path. We dispatch by
        // content-type so the same mock can answer JSON-RPC (existing tests)
        // and gRPC (suibase-daemon's primary protocol post-refactor).
        let state = self.params.state.clone();
        let app = Router::new()
            .fallback(any(unified_handler))
            .with_state(state);

        // Define the address to serve on
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], self.params.state.port));
        log::info!("{} listening on {}", self.task_name, addr);

        // Start cache cleanup task
        let state_for_cleanup = self.params.state.clone();
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Clean every minute
            loop {
                interval.tick().await;
                state_for_cleanup.cleanup_cache();
            }
        });

        // Run the server
        let server_result = axum_server::Server::bind(addr)
            .serve(app.into_make_service())
            .await;

        // Clean up the cleanup task when server stops
        cleanup_task.abort();

        server_result.map_err(|e| anyhow::anyhow!("Mock server error: {}", e))?;
        Ok(())
    }
}

/// Single entry point — dispatches by `content-type`.
///
/// - `application/grpc*` → answer like sui-node would (or like a non-gRPC
///   upstream, per `MockServerBehavior::respond_non_grpc`).
/// - anything else (default JSON-RPC) → existing JSON-RPC handler logic.
async fn unified_handler(
    State(state): State<Arc<MockServerState>>,
    req: axum::extract::Request,
) -> Response {
    let is_grpc = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.starts_with("application/grpc"))
        .unwrap_or(false);

    if is_grpc {
        // Deliberately DO NOT buffer the request body on the gRPC path. A
        // client/bidi-streaming client (e.g. server reflection — what grpcurl
        // uses) keeps its request stream open, so buffering here would hang
        // until the client half-closes. Answering without reading the body
        // mirrors a real bidi server that responds to each request message as
        // it arrives, and lets a test prove the proxy pipes the request body
        // through instead of buffering it (see proxy_grpc_request_streaming).
        handle_grpc_request(state).await
    } else {
        // JSON path: buffer the (small) body then hand off.
        let bytes = match axum::body::to_bytes(req.into_body(), 16 * 1024 * 1024).await {
            Ok(b) => b,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        let parsed: Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        match handle_jsonrpc_request_inner(state, parsed).await {
            Ok(json) => json.into_response(),
            Err(status) => status.into_response(),
        }
    }
}

/// Mock a sui-node gRPC unary response.
///
/// Honors `MockServerBehavior::respond_non_grpc`: when set, answer as a
/// JSON-RPC-only gateway would (HTTP 200 + content-type: application/json),
/// which is what the suibase-daemon proxy treats as `NOT_GRPC_CAPABLE`.
async fn handle_grpc_request(state: Arc<MockServerState>) -> Response {
    // Record the request in statistics so gRPC traffic flows through the
    // same counters as JSON-RPC in mock stats.
    {
        let mut stats = state.stats.write().unwrap();
        stats.inc_request();
    }

    // Behavior knobs we honor for gRPC (kept minimal — the rich JSON behavior
    // set isn't needed for capability tests).
    let behavior = state.get_behavior();

    // Rate limit check FIRST (match the JSON-RPC handler order) — rate-limited
    // requests must short-circuit before the artificial latency sleep.
    // Otherwise a (latency_ms=2000, rate_limit=1qps) configuration burns the
    // full latency on every rejected request, distorting timing-sensitive
    // tests that compare protocols.
    if state.check_rate_limit() {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    // Latency simulation.
    if behavior.latency_ms > 0 {
        sleep(Duration::from_millis(behavior.latency_ms as u64)).await;
        let mut stats = state.stats.write().unwrap();
        stats.inc_delay(behavior.latency_ms);
    }

    // Simulate failure (mirrors the JSON-RPC handler so existing
    // proxy/rate-limit tests that drive a server "down" by setting
    // failure_rate = 1.0 keep working now that the probe is gRPC).
    if behavior.failure_rate > 0.0 {
        let r: f64 = rand::random();
        if r < behavior.failure_rate {
            // Increment failure stats in a tight scope; explicit drop so the
            // RwLock guard never crosses the await below (which would make
            // the future !Send and reject the axum handler).
            {
                let mut stats = state.stats.write().unwrap();
                stats.inc_failure();
                if matches!(behavior.error_type, Some(MockErrorType::RateLimited)) {
                    stats.inc_rate_limit();
                }
            }
            return match behavior.error_type.as_ref() {
                Some(MockErrorType::Timeout) => {
                    sleep(Duration::from_secs(5)).await;
                    StatusCode::REQUEST_TIMEOUT.into_response()
                }
                Some(MockErrorType::ConnectionRefused) => {
                    StatusCode::SERVICE_UNAVAILABLE.into_response()
                }
                Some(MockErrorType::InternalError) | None => {
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
                Some(MockErrorType::RateLimited) => StatusCode::TOO_MANY_REQUESTS.into_response(),
            };
        }
    }

    // Capability flip — simulate a non-gRPC upstream.
    if behavior.respond_non_grpc {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"result":"ok"}"#))
            .unwrap();
    }

    // Simulate a server-streaming gRPC method (e.g. SubscribeCheckpoints):
    // emit an empty DATA frame every 100 ms and NEVER send END_STREAM.
    // A correctly-implemented proxy must stream this body through to the
    // client; a proxy that calls BodyExt::collect on the response will hang.
    if behavior.grpc_stream_forever {
        let body_stream = futures::stream::unfold((), |()| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            // 5-byte empty gRPC frame (compression flag=0, length=0).
            Some((
                Ok::<_, std::io::Error>(Bytes::from_static(&[0u8, 0, 0, 0, 0])),
                (),
            ))
        });
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, HeaderValue::from_static("application/grpc"))
            .body(Body::from_stream(body_stream))
            .unwrap();
    }

    // Simulate a misbehaving upstream that compresses despite the proxy's
    // `grpc-accept-encoding: identity`: a normal unary gRPC response tagged
    // `grpc-encoding: gzip`. The Sui CLI's tonic client can't decode that, so
    // the proxy must detect the encoding on the response headers and retry /
    // reject rather than forward it verbatim. Setting the header is enough to
    // trip the proxy's detection (it inspects the response `grpc-encoding`).
    if behavior.grpc_compress_response {
        let empty_frame: Bytes = Bytes::from_static(&[0u8, 0, 0, 0, 0]);
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, HeaderValue::from_static("application/grpc"))
            .header("grpc-encoding", "gzip")
            .header("grpc-status", "0")
            .body(Body::from(empty_frame))
            .unwrap();
    }

    // Minimal valid gRPC unary response:
    //   * status 200
    //   * content-type: application/grpc
    //   * one 5-byte length-prefixed data frame containing an empty message
    //     (compression flag=0, length=0). Real services follow with trailers
    //     carrying grpc-status; we use the "trailers-as-headers" shortcut
    //     (header-only trailers): grpc-status: 0 in regular headers. The
    //     proxy's check is just (200 + content-type starts with application/
    //     grpc), so this is enough for capability/health tests.
    let empty_frame: Bytes = Bytes::from_static(&[0u8, 0, 0, 0, 0]);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static("application/grpc"))
        .header("grpc-status", "0")
        .body(Body::from(empty_frame))
        .unwrap()
}

/// Handler for JSON-RPC requests. The body parse moved up to `unified_handler`;
/// this function now takes the already-parsed JSON.
async fn handle_jsonrpc_request_inner(
    state: Arc<MockServerState>,
    request: Value,
) -> Result<Json<Value>, StatusCode> {
    // Record the request in statistics
    {
        let mut stats = state.stats.write().unwrap();
        stats.inc_request();
    }

    // Check rate limiting first (from Link configuration)
    if state.check_rate_limit() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Get current behavior configuration
    let behavior = state.get_behavior();

    // Apply artificial latency if configured  
    if behavior.latency_ms > 0 {
        let delay_duration = Duration::from_millis(behavior.latency_ms as u64);
        sleep(delay_duration).await;
        
        // Record the delay in statistics
        let mut stats = state.stats.write().unwrap();
        stats.inc_delay(behavior.latency_ms);
    }

    // Check if we should simulate a failure
    if behavior.failure_rate > 0.0 {
        let random_value: f64 = rand::random();
        if random_value < behavior.failure_rate {
            // Record the failure in statistics
            {
                let mut stats = state.stats.write().unwrap();
                stats.inc_failure();
            }

            // Return appropriate error based on error_type
            return match behavior.error_type.as_ref() {
                Some(MockErrorType::Timeout) => {
                    // Simulate timeout by waiting then returning an error
                    sleep(Duration::from_secs(5)).await;
                    Err(StatusCode::REQUEST_TIMEOUT)
                }
                Some(MockErrorType::ConnectionRefused) => {
                    Err(StatusCode::SERVICE_UNAVAILABLE)
                }
                Some(MockErrorType::InternalError) => {
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
                Some(MockErrorType::RateLimited) => {
                    let mut stats = state.stats.write().unwrap();
                    stats.inc_rate_limit();
                    Err(StatusCode::TOO_MANY_REQUESTS)
                }
                None => {
                    // Generic failure
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            };
        }
    }

    // If we have a custom response body, use it
    if let Some(custom_response) = behavior.response_body {
        return Ok(Json(custom_response));
    }

    // If proxy is enabled, try to proxy to localnet with caching
    if behavior.proxy_enabled {
        match handle_proxy_request(&state, &request, behavior.cache_ttl_secs).await {
            Ok(response) => return Ok(Json(response)),
            Err(e) => {
                log::warn!("Proxy failed for {}, falling back to default response: {}", state.alias, e);
                // Fall through to default response
            }
        }
    }

    // Otherwise, generate a default successful JSON-RPC response
    let response = create_default_jsonrpc_response(&request);
    Ok(Json(response))
}

/// Handle proxy request with caching
async fn handle_proxy_request(
    state: &Arc<MockServerState>,
    request: &Value,
    cache_ttl_secs: u64,
) -> Result<Value, String> {
    // Generate cache key
    let cache_key = state.cache_key(request);
    
    // Try to get response from cache first
    if let Some(cached_response) = state.get_cached_response(&cache_key) {
        // Record cache hit
        if let Ok(mut stats) = state.stats.write() {
            stats.inc_cache_hit();
        }
        
        // Restore the original request ID in the response
        let mut response = cached_response;
        if let Some(request_id) = request.get("id") {
            if let Some(response_obj) = response.as_object_mut() {
                response_obj.insert("id".to_string(), request_id.clone());
            }
        }
        
        return Ok(response);
    }
    
    // Cache miss - proxy to localnet
    if let Ok(mut stats) = state.stats.write() {
        stats.inc_cache_miss();
    }
    
    match state.proxy_to_localnet(request).await {
        Ok(response) => {
            // Cache the response (remove ID before caching)
            let mut cacheable_response = response.clone();
            if let Some(response_obj) = cacheable_response.as_object_mut() {
                response_obj.remove("id"); // Remove ID for caching
            }
            
            state.cache_response(cache_key, cacheable_response, cache_ttl_secs);
            
            Ok(response)
        }
        Err(e) => Err(e),
    }
}

/// Create a default successful JSON-RPC response
fn create_default_jsonrpc_response(request: &Value) -> Value {
    // Extract the ID from the request, defaulting to null if not present
    let id = request.get("id").cloned().unwrap_or(json!(null));
    
    // Extract the method to provide method-specific responses
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("unknown");
    
    // Provide realistic responses for common Sui methods
    let result = match method {
        "sui_getLatestSuiSystemState" => {
            json!({
                "epoch": "100",
                "protocolVersion": "1",
                "systemStateVersion": "1",
                "storageFundTotalObjectStorageRebates": "0",
                "storageFundNonRefundableBalance": "0",
                "referenceGasPrice": "1000",
                "safeMode": false,
                "safeModeStorageRewards": "0",
                "safeModeComputationRewards": "0",
                "safeModeStorageRebates": "0",
                "safeModeNonRefundableStorageFee": "0",
                "epochStartTimestampMs": "1640995200000",
                "epochDurationMs": "86400000",
                "stakeSubsidyStartEpoch": "0",
                "maxValidatorCount": "150",
                "minValidatorJoiningStake": "30000000000000",
                "validatorLowStakeThreshold": "20000000000000",
                "validatorVeryLowStakeThreshold": "15000000000000",
                "validatorLowStakeGracePeriod": "5",
                "stakeSubsidyBalance": "0",
                "stakeSubsidyDistributionCounter": "0",
                "stakeSubsidyCurrentDistributionAmount": "0",
                "stakeSubsidyPeriodLength": "10",
                "stakeSubsidyDecreaseRate": "1000",
                "totalStake": "1000000000000000",
                "activeValidators": [],
                "pendingActiveValidatorsId": "0x0",
                "pendingActiveValidatorsSize": "0",
                "pendingRemovals": [],
                "stakingPoolMappingsId": "0x0",
                "stakingPoolMappingsSize": "0",
                "inactiveValidatorsId": "0x0",
                "inactiveValidatorsSize": "0",
                "validatorCandidatesId": "0x0",
                "validatorCandidatesSize": "0",
                "atRiskValidators": [],
                "validatorReportRecords": []
            })
        }
        "sui_getObject" => {
            json!({
                "objectId": "0x123456789abcdef",
                "version": "1",
                "digest": "mock_digest_hash",
                "type": "0x2::coin::Coin<0x2::sui::SUI>",
                "owner": {
                    "AddressOwner": "0xabcdef123456789"
                },
                "previousTransaction": "mock_tx_digest",
                "storageRebate": "100",
                "content": {
                    "dataType": "moveObject",
                    "type": "0x2::coin::Coin<0x2::sui::SUI>",
                    "hasPublicTransfer": true,
                    "fields": {
                        "balance": "1000000000",
                        "id": {
                            "id": "0x123456789abcdef"
                        }
                    }
                }
            })
        }
        "sui_getCheckpoints" => {
            json!({
                "data": [],
                "nextCursor": null,
                "hasNextPage": false
            })
        }
        "sui_getBalance" => {
            json!({
                "coinType": "0x2::sui::SUI",
                "coinObjectCount": 5,
                "totalBalance": "5000000000",
                "lockedBalance": {}
            })
        }
        _ => {
            // Generic successful response for unknown methods
            json!({
                "status": "success",
                "data": "mock_response_data",
                "timestamp": chrono::Utc::now().timestamp()
            })
        }
    };

    json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id
    })
}