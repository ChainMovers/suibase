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
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
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
        // Create the axum router with our JSON-RPC handler
        let app = Router::new()
            .route("/", post(handle_jsonrpc_request))
            .with_state(self.params.state.clone());

        // Define the address to serve on
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], self.params.state.port));
        log::info!("{} listening on {}", self.task_name, addr);

        // Run the server
        axum_server::Server::bind(addr)
            .serve(app.into_make_service())
            .await
            .map_err(|e| anyhow::anyhow!("Mock server error: {}", e))?;

        Ok(())
    }
}

/// Handler for JSON-RPC requests
async fn handle_jsonrpc_request(
    State(state): State<Arc<MockServerState>>,
    Json(request): Json<Value>,
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

    // Otherwise, generate a default successful JSON-RPC response
    let response = create_default_jsonrpc_response(&request);
    Ok(Json(response))
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