// Mock server types for testing suibase-daemon proxy server functionality.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::rate_limiter::RateLimiter;
use common::shared_types::Link;


/// Behavior configuration for a mock server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockServerBehavior {
    /// Probability of request failure (0.0 to 1.0)
    #[serde(default)]
    pub failure_rate: f64,
    
    /// Additional latency to add to responses in milliseconds
    #[serde(default)]
    pub latency_ms: u32,
    
    /// HTTP status code to return for successful responses
    #[serde(default = "default_http_status")]
    pub http_status: u16,
    
    /// Type of error to simulate when failing
    #[serde(default)]
    pub error_type: Option<MockErrorType>,
    
    /// Custom JSON response body to return instead of default
    #[serde(default)]
    pub response_body: Option<serde_json::Value>,
}

impl Default for MockServerBehavior {
    fn default() -> Self {
        Self {
            failure_rate: 0.0,
            latency_ms: 0,
            http_status: 200,
            error_type: None,
            response_body: None,
        }
    }
}

fn default_http_status() -> u16 {
    200
}

/// Types of errors that can be simulated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MockErrorType {
    Timeout,
    ConnectionRefused,
    InternalError,
    RateLimited,
}

/// Statistics for a single mock server
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MockServerStats {
    /// Total number of requests received
    pub requests_received: u64,
    
    /// Number of requests that resulted in simulated failure
    pub requests_failed: u64,
    
    /// Number of requests that had simulated delay
    pub requests_delayed: u64,
    
    /// Total delay added across all requests in milliseconds
    pub total_delay_ms: u64,
    
    /// Number of rate limit simulations triggered
    pub rate_limit_hits: u64,
    
    /// Number of times behavior was changed
    pub behavior_changes: u64,
}

impl MockServerStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn clear(&mut self) {
        *self = Self::new();
    }
    
    /// Get average delay per request in milliseconds
    pub fn average_delay_ms(&self) -> f64 {
        if self.requests_delayed == 0 {
            0.0
        } else {
            self.total_delay_ms as f64 / self.requests_delayed as f64
        }
    }
    
    /// Record a new request
    pub fn inc_request(&mut self) {
        self.requests_received += 1;
    }
    
    /// Record a failed request
    pub fn inc_failure(&mut self) {
        self.requests_failed += 1;
    }
    
    /// Record a delayed request
    pub fn inc_delay(&mut self, delay_ms: u32) {
        self.requests_delayed += 1;
        self.total_delay_ms += delay_ms as u64;
    }
    
    /// Record a rate limit hit
    pub fn inc_rate_limit(&mut self) {
        self.rate_limit_hits += 1;
    }
    
    /// Record a behavior change
    pub fn inc_behavior_change(&mut self) {
        self.behavior_changes += 1;
    }
}

/// State for a single mock server instance
pub struct MockServerState {
    /// Server alias (e.g., "mock-0")
    pub alias: String,
    
    /// Port the mock server is listening on
    pub port: u16,
    
    /// Current behavior configuration
    pub behavior: Arc<RwLock<MockServerBehavior>>,
    
    /// Runtime statistics
    pub stats: Arc<RwLock<MockServerStats>>,
    
    /// Rate limiter for enforcing max_per_secs/max_per_min from Link config
    pub rate_limiter: Arc<RwLock<Option<RateLimiter>>>,
    
    /// Handle to the async task running the server
    pub handle: Option<SubsystemHandle>,
}

impl std::fmt::Debug for MockServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockServerState")
            .field("alias", &self.alias)
            .field("port", &self.port)
            .field("behavior", &self.behavior)
            .field("stats", &self.stats)
            .field("rate_limiter", &"Arc<RwLock<Option<RateLimiter>>>")
            .field("handle", &"SubsystemHandle")
            .finish()
    }
}

impl MockServerState {
    pub fn new(alias: String, port: u16) -> Self {
        Self {
            alias,
            port,
            behavior: Arc::new(RwLock::new(MockServerBehavior::default())),
            stats: Arc::new(RwLock::new(MockServerStats::new())),
            rate_limiter: Arc::new(RwLock::new(None)),
            handle: None,
        }
    }
    
    /// Update the behavior of this mock server
    pub fn set_behavior(&self, new_behavior: MockServerBehavior) {
        if let Ok(mut behavior) = self.behavior.write() {
            *behavior = new_behavior;
        }
        
        // Record the behavior change
        if let Ok(mut stats) = self.stats.write() {
            stats.inc_behavior_change();
        }
    }
    
    /// Get a copy of the current behavior
    pub fn get_behavior(&self) -> MockServerBehavior {
        self.behavior.read().unwrap().clone()
    }
    
    /// Clear statistics
    pub fn clear_stats(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.clear();
        }
    }
    
    /// Get a copy of current statistics
    pub fn get_stats(&self) -> MockServerStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Reset behavior to default
    pub fn reset_behavior(&self) {
        self.set_behavior(MockServerBehavior::default());
    }
    
    /// Update rate limiter configuration from Link config
    pub fn update_rate_limiter(&self, link_config: &Link) {
        let new_rate_limiter = Self::create_rate_limiter_from_config(link_config);
        if let Ok(mut rate_limiter) = self.rate_limiter.write() {
            *rate_limiter = new_rate_limiter;
            log::debug!("Updated rate limiter for mock server {}", self.alias);
        }
    }
    
    /// Create rate limiter from Link configuration (similar to TargetServer)
    fn create_rate_limiter_from_config(config: &Link) -> Option<RateLimiter> {
        // Only create a rate limiter if at least one limit is configured (including 0 for unlimited)
        if config.max_per_secs.is_some() || config.max_per_min.is_some() {
            let max_per_secs = config.max_per_secs.unwrap_or(0);
            let max_per_min = config.max_per_min.unwrap_or(0);

            match RateLimiter::new(max_per_secs, max_per_min) {
                Ok(limiter) => {
                    log::debug!(
                        "Created rate limiter for mock server {}: max_per_secs={}, max_per_min={}",
                        config.alias, max_per_secs, max_per_min
                    );
                    Some(limiter)
                }
                Err(err) => {
                    log::warn!(
                        "Failed to create rate limiter for mock server {}: {}",
                        config.alias,
                        err
                    );
                    None
                }
            }
        } else {
            log::debug!("No rate limits configured for mock server {}", config.alias);
            None
        }
    }
    
    /// Check if a request should be rate limited (returns true if rate limit exceeded)
    pub fn check_rate_limit(&self) -> bool {
        if let Ok(mut rate_limiter_guard) = self.rate_limiter.write() {
            if let Some(ref mut rate_limiter) = *rate_limiter_guard {
                // Try to consume a token - if it fails, we're rate limited
                if rate_limiter.try_acquire_token().is_err() {
                    // Record rate limit hit in stats
                    if let Ok(mut stats) = self.stats.write() {
                        stats.inc_rate_limit();
                    }
                    return true;
                }
            }
        }
        false
    }
}

/// Request to control mock server behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockServerControlRequest {
    /// Alias of the mock server to control
    pub alias: String,
    
    /// New behavior to apply
    pub behavior: MockServerBehavior,
}

/// Response containing mock server statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockServerStatsResponse {
    /// Alias of the mock server
    pub alias: String,
    
    /// Current statistics
    pub stats: MockServerStats,
    
    /// Whether stats were reset after reading
    pub reset: bool,
}

impl MockServerStatsResponse {
    pub fn new(alias: String, stats: MockServerStats, reset: bool) -> Self {
        Self {
            alias,
            stats,
            reset,
        }
    }
}


/// Request to control multiple mock servers in batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockServerBatchRequest {
    /// List of control requests to apply
    pub requests: Vec<MockServerControlRequest>,
}

/// Events for mock server internal communication
pub mod actions {
    use super::*;
    
    /// Actions that can be sent to mock server workers
    #[derive(Debug, Clone)]
    pub enum MockServerAction {
        /// Update server behavior
        UpdateBehavior(MockServerBehavior),
        
        /// Clear statistics
        ClearStats,
        
        /// Get current statistics
        GetStats,
    }
}

// Event constants for mock server communication
pub const EVENT_MOCK_SERVER_CONTROL: u8 = 128;
pub const EVENT_MOCK_SERVER_STATS: u8 = 129;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_server_behavior_default() {
        let behavior = MockServerBehavior::default();
        assert_eq!(behavior.failure_rate, 0.0);
        assert_eq!(behavior.latency_ms, 0);
        assert_eq!(behavior.http_status, 200);
        assert!(behavior.error_type.is_none());
        assert!(behavior.response_body.is_none());
    }

    #[test]
    fn test_mock_server_stats() {
        let mut stats = MockServerStats::new();
        
        // Test initial state
        assert_eq!(stats.requests_received, 0);
        assert_eq!(stats.requests_failed, 0);
        assert_eq!(stats.requests_delayed, 0);
        assert_eq!(stats.total_delay_ms, 0);
        assert_eq!(stats.rate_limit_hits, 0);
        assert_eq!(stats.behavior_changes, 0);
        
        // Test incrementing
        stats.inc_request();
        stats.inc_failure();
        stats.inc_delay(100);
        stats.inc_rate_limit();
        stats.inc_behavior_change();
        
        assert_eq!(stats.requests_received, 1);
        assert_eq!(stats.requests_failed, 1);
        assert_eq!(stats.requests_delayed, 1);
        assert_eq!(stats.total_delay_ms, 100);
        assert_eq!(stats.rate_limit_hits, 1);
        assert_eq!(stats.behavior_changes, 1);
        
        // Test average delay calculation
        stats.inc_delay(200);
        assert_eq!(stats.average_delay_ms(), 150.0); // (100 + 200) / 2
        
        // Test clear
        stats.clear();
        assert_eq!(stats.requests_received, 0);
        assert_eq!(stats.average_delay_ms(), 0.0);
    }

    #[test]
    fn test_mock_server_state() {
        let state = MockServerState::new("mock-0".to_string(), 50001);
        
        assert_eq!(state.alias, "mock-0");
        assert_eq!(state.port, 50001);
        
        // Test behavior management
        let new_behavior = MockServerBehavior {
            failure_rate: 0.5,
            latency_ms: 100,
            http_status: 500,
            error_type: Some(MockErrorType::InternalError),
            response_body: None,
        };
        
        state.set_behavior(new_behavior.clone());
        let retrieved_behavior = state.get_behavior();
        
        assert_eq!(retrieved_behavior.failure_rate, 0.5);
        assert_eq!(retrieved_behavior.latency_ms, 100);
        assert_eq!(retrieved_behavior.http_status, 500);
        
        // Test stats management
        let stats = state.get_stats();
        assert_eq!(stats.behavior_changes, 1); // Set from set_behavior call
        
        state.clear_stats();
        let cleared_stats = state.get_stats();
        assert_eq!(cleared_stats.behavior_changes, 0);
    }

    #[test]
    fn test_mock_server_serialization() {
        let behavior = MockServerBehavior {
            failure_rate: 0.75,
            latency_ms: 250,
            http_status: 429,
            error_type: Some(MockErrorType::RateLimited),
            response_body: Some(serde_json::json!({"error": "rate limited"})),
        };
        
        // Test serialization/deserialization
        let json = serde_json::to_string(&behavior).unwrap();
        let deserialized: MockServerBehavior = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.failure_rate, 0.75);
        assert_eq!(deserialized.latency_ms, 250);
        assert_eq!(deserialized.http_status, 429);
        assert!(matches!(deserialized.error_type, Some(MockErrorType::RateLimited)));
        assert!(deserialized.response_body.is_some());
    }
}