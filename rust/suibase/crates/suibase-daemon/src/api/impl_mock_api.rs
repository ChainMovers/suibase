// Implementation of Mock API for controlling mock servers during testing.

use axum::async_trait;
use jsonrpsee::core::RpcResult;

use crate::mock_server_manager::MockServerManager;
use crate::shared_types::{GlobalsProxyMT, MockServerBehavior, MockServerStatsResponse};

use super::{MockApiServer, SuccessResponse, RpcInputError, RpcSuibaseError};
use common::basic_types::AdminControllerTx;

pub struct MockApiImpl {
    #[allow(dead_code)]
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
    ) -> RpcResult<MockServerStatsResponse> {
        // Validate that the alias is a mock server
        if !MockServerManager::is_mock_server(&alias) {
            return Err(RpcInputError::InvalidParams(
                "alias".to_string(),
                format!("'{}' is not a mock server (must start with 'mock-')", alias),
            ).into());
        }

        // Read-only stats access - request from AdminController without reset
        {
            // Read-only stats access - need to request from AdminController without reset
            use common::basic_types::{AdminControllerMsg, EVENT_MOCK_SERVER_STATS};
            
            let mut msg = AdminControllerMsg::new();
            msg.event_id = EVENT_MOCK_SERVER_STATS;
            msg.data_string = Some(alias.clone());
            
            let (tx, rx) = tokio::sync::oneshot::channel();
            msg.resp_channel = Some(tx);
            
            match self.admctrl_tx.send(msg).await {
                Ok(_) => {
                    match rx.await {
                        Ok(response) => {
                            // Response format: "stats_json|behavior_json"
                            log::debug!("Raw response from AdminController: {}", response);
                            let parts: Vec<&str> = response.splitn(2, '|').collect();
                            log::debug!("Split into {} parts", parts.len());
                            
                            match serde_json::from_str::<crate::shared_types::MockServerStats>(parts[0]) {
                                Ok(stats) => {
                                    let mut response = MockServerStatsResponse::new(alias, stats, false);
                                    
                                    // Parse behavior if present
                                    if parts.len() > 1 {
                                        if let Ok(behavior) = serde_json::from_str::<crate::shared_types::MockServerBehavior>(parts[1]) {
                                            response = response.with_behavior(behavior);
                                        }
                                    }
                                    
                                    log::debug!("Retrieved read-only stats for mock server: {}", response.alias);
                                    Ok(response)
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
                    log::warn!("Failed to send stats request for mock server '{}': {}", alias, e);
                    Err(RpcSuibaseError::InfoError(format!("Failed to send mock server stats request: {}", e)).into())
                }
            }
        }
    }

    async fn mock_server_reset(
        &self,
        alias: String,
    ) -> RpcResult<SuccessResponse> {
        let mut resp = SuccessResponse::new();
        resp.header.method = "mockServerReset".to_string();
        
        // Validate that the alias is a mock server
        if !MockServerManager::is_mock_server(&alias) {
            return Err(RpcInputError::InvalidParams(
                "alias".to_string(),
                format!("'{}' is not a mock server (must start with 'mock-')", alias),
            ).into());
        }

        // Send reset message to AdminController
        use common::basic_types::{AdminControllerMsg, EVENT_MOCK_SERVER_RESET};
        
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_MOCK_SERVER_RESET;
        msg.data_string = Some(alias.clone());
        
        match self.admctrl_tx.send(msg).await {
            Ok(_) => {
                resp.result = true;
                resp.info = Some(format!("Successfully reset stats for mock server '{}'", alias));
                log::debug!("Mock server '{}' stats reset successfully", alias);
            }
            Err(e) => {
                resp.result = false;
                resp.info = Some(format!("Failed to reset stats for mock server '{}': {}", alias, e));
                log::warn!("Failed to send reset message for mock server '{}': {}", alias, e);
            }
        }

        Ok(resp)
    }

}