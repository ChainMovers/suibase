// Mock server manager for lifecycle management of mock servers.
//
// Manages the lifecycle of mock server async tasks and provides control
// interfaces for test scenarios.

use crate::shared_types::{GlobalsProxyMT, MockServerState, MockServerBehavior, MockServerStats, MockServerControlRequest};
use crate::workers::{MockServerWorker, MockServerParams};

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle, SubsystemBuilder};

/// Message types for MockServerManager subsystem
#[derive(Debug)]
pub enum MockServerMsg {
    /// Process configuration changes (start/stop servers based on config)
    ConfigUpdate {
        workdir_name: String,
    },
    /// Control individual server behavior
    ServerControl {
        alias: String,
        behavior: MockServerBehavior,
    },
    /// Control multiple servers at once
    ServerBatchControl {
        requests: Vec<MockServerControlRequest>,
    },
    /// Reset statistics for a specific server (mutable operation)
    ServerStatsReset {
        alias: String,
        response_channel: tokio::sync::oneshot::Sender<Result<MockServerStats>>,
    },
}

pub type MockServerTx = tokio::sync::mpsc::Sender<MockServerMsg>;
pub type MockServerRx = tokio::sync::mpsc::Receiver<MockServerMsg>;

/// Parameters for MockServerManager subsystem
#[derive(Clone, Debug)]
pub struct MockServerManagerParams {
    globals: GlobalsProxyMT,
}

impl MockServerManagerParams {
    pub fn new(globals: GlobalsProxyMT) -> Self {
        Self { globals }
    }
}

/// Manager for all mock server instances
#[derive(Debug)]
pub struct MockServerManager {
    /// Subsystem parameters
    params: MockServerManagerParams,
    /// Map of alias -> mock server state
    mock_servers: Arc<RwLock<HashMap<String, Arc<MockServerState>>>>,
    /// Message receiver for subsystem communication
    mock_server_rx: MockServerRx,
}

impl MockServerManager {
    pub fn new(params: MockServerManagerParams, mock_server_rx: MockServerRx) -> Self {
        Self {
            params,
            mock_servers: Arc::new(RwLock::new(HashMap::new())),
            mock_server_rx,
        }
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            let cur_msg = self.mock_server_rx.recv().await;
            if cur_msg.is_none() || subsys.is_shutdown_requested() {
                // Channel closed or shutdown requested.
                return;
            }
            if let Err(e) = self.handle_message(cur_msg.unwrap(), subsys).await {
                log::error!("MockServerManager message handling error: {}", e);
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        // The loop to handle all incoming messages.
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

    /// Handle individual messages
    async fn handle_message(&mut self, msg: MockServerMsg, subsys: &SubsystemHandle) -> Result<()> {
        match msg {
            MockServerMsg::ConfigUpdate { workdir_name } => {
                self.handle_config_update(&workdir_name, subsys).await
            }
            MockServerMsg::ServerControl { alias, behavior } => {
                self.handle_server_control(&alias, behavior).await
            }
            MockServerMsg::ServerBatchControl { requests } => {
                self.handle_server_batch_control(&requests).await
            }
            MockServerMsg::ServerStatsReset { alias, response_channel } => {
                let result = self.handle_server_stats_reset(&alias).await;
                let _ = response_channel.send(result); // Ignore send error if receiver dropped
                Ok(())
            }
        }
    }

    /// Handle configuration update message
    async fn handle_config_update(&mut self, workdir_name: &str, subsys: &SubsystemHandle) -> Result<()> {
        log::debug!("MockServerManager::handle_config_update for workdir: {}", workdir_name);
        
        // Only process localnet workdir for now (as per the spec)
        if workdir_name != "localnet" {
            return Ok(());
        }

        // Read the InputPort from globals and extract server info within lock scope
        let mut current_config = Vec::new();
        
        {
            let globals = self.params.globals.read().await;
            let input_port = match globals.find_input_port_by_name(workdir_name) {
                Some(port) => port,
                None => {
                    log::warn!("No input port found for workdir: {}", workdir_name);
                    return Ok(());
                }
            };

            // Find all links with "mock-" prefix and extract their configuration
            for (_, target_server) in input_port.target_servers.iter() {
                let alias = target_server.alias();
                if alias.starts_with("mock-") {
                    let rpc = target_server.rpc();
                    if let Ok(url) = reqwest::Url::parse(&rpc) {
                        if let Some(port) = url.port() {
                            let config = target_server.get_config();
                            current_config.push((alias.clone(), port, config.clone()));
                        } else {
                            log::warn!("Mock server {} has no port in RPC URL: {}", alias, rpc);
                        }
                    } else {
                        log::warn!("Mock server {} has invalid RPC URL: {}", alias, rpc);
                    }
                }
            }
        } // Release globals lock

        let mut mock_servers = self.mock_servers.write().unwrap();
        
        // Build a set of current aliases for easy lookup
        let current_aliases: std::collections::HashSet<String> = 
            current_config.iter().map(|(alias, _, _)| alias.clone()).collect();
        
        // Stop and remove servers that are no longer in the configuration
        let running_aliases: Vec<String> = mock_servers.keys().cloned().collect();
        for alias in running_aliases {
            if !current_aliases.contains(&alias) {
                log::info!("Stopping mock server {} (removed from config)", alias);
                mock_servers.remove(&alias);
                // Note: The SubsystemHandle will handle graceful shutdown automatically
            }
        }
        
        // Process current configuration: start new servers and update existing ones
        for (alias, port, link_config) in current_config {
            if mock_servers.contains_key(&alias) {
                // Server exists - check if configuration needs updating
                if let Some(state) = mock_servers.get(&alias) {
                    // Check if port changed (requires restart)
                    if state.port != port {
                        log::info!("Mock server {} port changed from {} to {} - restarting", alias, state.port, port);
                        mock_servers.remove(&alias);
                        // Will be recreated below
                    } else {
                        // Port is same - server can continue running
                        // Update configuration parameters like rate limits without restart
                        state.update_rate_limiter(&link_config);
                        log::debug!("Mock server {} configuration updated (no restart needed)", alias);
                        continue;
                    }
                }
            }
            
            // Start new server (either completely new or restarted due to port change)
            if !mock_servers.contains_key(&alias) {
                // Create mock server state
                let state = Arc::new(MockServerState::new(alias.clone(), port));
                
                // Initialize rate limiter from Link configuration
                state.update_rate_limiter(&link_config);
                
                // Create and start the worker
                let params = MockServerParams::new(state.clone());
                let worker = MockServerWorker::new(params);
                
                // Start the worker as a subsystem
                let _nested = subsys.start(SubsystemBuilder::new(
                    format!("MockServer-{}", alias),
                    |handle| worker.run(handle)
                ));
                
                log::info!("Started mock server {} on port {} with rate limits", alias, port);
                
                // Store the state
                mock_servers.insert(alias, state);
            }
        }

        Ok(())
    }


    /// Handle individual server control message
    async fn handle_server_control(&mut self, alias: &str, behavior: MockServerBehavior) -> Result<()> {
        let mock_servers = self.mock_servers.read().unwrap();
        
        let state = mock_servers
            .get(alias)
            .ok_or_else(|| anyhow!("Mock server '{}' not found", alias))?;

        state.set_behavior(behavior);
        log::debug!("Set behavior for mock server {}", alias);

        Ok(())
    }

    /// Handle batch server control message
    async fn handle_server_batch_control(&mut self, requests: &[MockServerControlRequest]) -> Result<()> {
        for request in requests {
            self.handle_server_control(&request.alias, request.behavior.clone())
                .await?;
        }
        Ok(())
    }

    /// Get statistics for a specific mock server (read-only)
    /// For stats with reset, use messaging through AdminController
    pub fn get_server_stats(&self, alias: &str) -> Result<MockServerStats> {
        let mock_servers = self.mock_servers.read().unwrap();
        
        let state = mock_servers
            .get(alias)
            .ok_or_else(|| anyhow!("Mock server '{}' not found", alias))?;

        Ok(state.get_stats())
    }

    /// Get statistics for a specific mock server and reset (internal helper for message handling)
    async fn handle_server_stats_reset(&mut self, alias: &str) -> Result<MockServerStats> {
        let mock_servers = self.mock_servers.read().unwrap();
        
        let state = mock_servers
            .get(alias)
            .ok_or_else(|| anyhow!("Mock server '{}' not found", alias))?;

        let stats = state.get_stats();
        state.clear_stats();
        Ok(stats)
    }

    /// Get list of all mock servers
    pub fn get_server_list(&self) -> Vec<String> {
        let mock_servers = self.mock_servers.read().unwrap();
        mock_servers.keys().cloned().collect()
    }

    /// Check if a server is a mock server based on alias
    pub fn is_mock_server(alias: &str) -> bool {
        alias.starts_with("mock-")
    }

    /// Get the number of running mock servers
    pub fn server_count(&self) -> usize {
        let mock_servers = self.mock_servers.read().unwrap();
        mock_servers.len()
    }
}

// Note: Default implementation removed since MockServerManager now requires a channel
// Use new() with proper channel setup instead

// Extension trait is no longer needed since InputPort already has workdir_name() method