// Implementation of Mock API for controlling mock servers during testing.

use axum::async_trait;
use jsonrpsee::core::RpcResult;

use crate::mock_server_manager::MockServerManager;
use crate::shared_types::{GlobalsProxyMT, MockServerBehavior, MockServerControlRequest, MockServerStatsResponse};

use super::{MockApiServer, SuccessResponse, RpcInputError, RpcSuibaseError};
use common::basic_types::AdminControllerTx;

pub struct MockApiImpl {
    pub globals: GlobalsProxyMT,
    pub admctrl_tx: AdminControllerTx,
}

impl MockApiImpl {
    pub fn new(globals: GlobalsProxyMT, admctrl_tx: AdminControllerTx) -> Self {
        Self { globals, admctrl_tx }
    }
}

#[async_trait]
impl MockApiServer for MockApiImpl {
    async fn mock_server_control(
        &self,
        alias: String,
        behavior: MockServerBehavior,
    ) -> RpcResult<SuccessResponse> {
        let mut resp = SuccessResponse::new();
        resp.header.method = "mockServerControl".to_string();
        resp.header.key = Some(alias.clone());

        // Validate that the alias is a mock server
        if !MockServerManager::is_mock_server(&alias) {
            return Err(RpcInputError::InvalidParams(
                "alias".to_string(),
                format!("'{}' is not a mock server (must start with 'mock-')", alias),
            ).into());
        }

        // Send message to AdminController to control the mock server
        use common::basic_types::{AdminControllerMsg, EVENT_MOCK_SERVER_CONTROL};
        
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_MOCK_SERVER_CONTROL;
        msg.data_string = Some(format!("{}:{}", alias, serde_json::to_string(&behavior).unwrap_or_default()));
        
        match self.admctrl_tx.send(msg).await {
            Ok(_) => {
                resp.result = true;
                resp.info = Some(format!("Successfully sent control command for mock server '{}'", alias));
                log::debug!("Mock server control message sent: {}", alias);
            }
            Err(e) => {
                resp.result = false;
                resp.info = Some(format!("Failed to send control command for mock server '{}': {}", alias, e));
                log::warn!("Mock server control message failed: {}: {}", alias, e);
            }
        }

        Ok(resp)
    }

    async fn mock_server_stats(
        &self,
        alias: String,
        reset_after: Option<bool>,
    ) -> RpcResult<MockServerStatsResponse> {
        // Validate that the alias is a mock server
        if !MockServerManager::is_mock_server(&alias) {
            return Err(RpcInputError::InvalidParams(
                "alias".to_string(),
                format!("'{}' is not a mock server (must start with 'mock-')", alias),
            ).into());
        }

        let reset_stats = reset_after.unwrap_or(false);

        if reset_stats {
            // Reset stats requires messaging (mutable operation)
            use common::basic_types::{AdminControllerMsg, EVENT_MOCK_SERVER_STATS_RESET};
            
            let mut msg = AdminControllerMsg::new();
            msg.event_id = EVENT_MOCK_SERVER_STATS_RESET;
            msg.data_string = Some(alias.clone());
            
            let (tx, rx) = tokio::sync::oneshot::channel();
            msg.resp_channel = Some(tx);
            
            match self.admctrl_tx.send(msg).await {
                Ok(_) => {
                    match rx.await {
                        Ok(response) => {
                            // Parse the response as MockServerStats JSON
                            match serde_json::from_str::<crate::shared_types::MockServerStats>(&response) {
                                Ok(stats) => {
                                    log::debug!("Retrieved and reset stats for mock server: {}", alias);
                                    Ok(MockServerStatsResponse::new(alias, stats, reset_stats))
                                }
                                Err(e) => {
                                    log::warn!("Failed to parse stats response for mock server '{}': {}", alias, e);
                                    Err(RpcSuibaseError::InfoError(format!("Failed to parse mock server stats: {}", e)).into())
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to receive stats response for mock server '{}': {}", alias, e);
                            Err(RpcSuibaseError::InfoError(format!("Failed to get mock server stats response: {}", e)).into())
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to send stats reset message for mock server '{}': {}", alias, e);
                    Err(RpcSuibaseError::InfoError(format!("Failed to send mock server stats request: {}", e)).into())
                }
            }
        } else {
            // Read-only stats access through globals
            // TODO: Implement globals access to mock server stats
            // For now, return empty stats
            log::debug!("Retrieved read-only stats for mock server: {}", alias);
            Ok(MockServerStatsResponse::new(alias, Default::default(), reset_stats))
        }
    }

    async fn mock_server_batch(
        &self,
        servers: Vec<MockServerControlRequest>,
    ) -> RpcResult<SuccessResponse> {
        let mut resp = SuccessResponse::new();
        resp.header.method = "mockServerBatch".to_string();

        // Validate all server aliases are mock servers
        for request in &servers {
            if !MockServerManager::is_mock_server(&request.alias) {
                return Err(RpcInputError::InvalidParams(
                    "servers".to_string(),
                    format!("'{}' is not a mock server (must start with 'mock-')", request.alias),
                ).into());
            }
        }

        // Send batch control message to AdminController
        use common::basic_types::{AdminControllerMsg, EVENT_MOCK_SERVER_BATCH_CONTROL};
        
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_MOCK_SERVER_BATCH_CONTROL;
        msg.data_string = Some(serde_json::to_string(&servers).unwrap_or_default());
        
        match self.admctrl_tx.send(msg).await {
            Ok(_) => {
                resp.result = true;
                resp.info = Some(format!("Successfully sent batch control command for {} mock servers", servers.len()));
                log::debug!("Mock server batch control message sent: {} servers", servers.len());
            }
            Err(e) => {
                resp.result = false;
                resp.info = Some(format!("Failed to send batch control command: {}", e));
                log::warn!("Mock server batch control message failed: {}", e);
            }
        }

        Ok(resp)
    }
}