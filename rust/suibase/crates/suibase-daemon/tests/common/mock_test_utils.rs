// Test utilities and harness for mock server integration tests.
// 
// Design:
// - Smart daemon lifecycle: only restart when code changes or config needs change
// - Persistent config for debugging failed tests  
// - Shared daemon instance across tests for performance
// - Config change detection to minimize restarts
//
// ⚠️  CRITICAL SAFETY: SEQUENTIAL EXECUTION ENFORCED ⚠️
// This harness uses a global mutex (TEST_EXECUTION_LOCK) to ensure only one test
// runs at a time. This prevents race conditions because tests:
// 1. Share a single suibase-daemon process
// 2. Modify the same suibase.yaml configuration file
// 3. Change daemon state that would interfere with parallel tests
//
// The lock is held for the entire duration of each test to guarantee safety.

use anyhow::{anyhow, bail, Result};
use reqwest::Client;  
use serde_json::json;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use tokio::time::sleep;
use once_cell::sync::Lazy;

use suibase_daemon::shared_types::{MockServerBehavior, MockServerControlRequest, MockServerStatsResponse};
use suibase_daemon::api::def_methods::LinksResponse;

// Global state to track daemon lifecycle across tests
static DAEMON_STATE: Lazy<Arc<Mutex<DaemonState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(DaemonState::new()))
});

// Global test execution lock - CRITICAL: Ensures only one test runs at a time
// This prevents race conditions between tests that modify shared daemon state
static TEST_EXECUTION_LOCK: Lazy<Arc<Mutex<()>>> = Lazy::new(|| {
    Arc::new(Mutex::new(()))
});

#[derive(Debug, Clone, PartialEq)]
struct DaemonState {
    is_running: bool,
    last_restart_reason: Option<String>,
}

impl DaemonState {
    fn new() -> Self {
        Self {
            is_running: false,
            last_restart_reason: None,
        }
    }
}

/// Test harness for managing suibase-daemon lifecycle and configuration during tests
pub struct MockServerTestHarness {
    /// HTTP client for API calls
    api_client: Client,
    
    /// Path to the suibase.yaml file
    yaml_path: PathBuf,
    
    /// Base URL for API calls
    api_base_url: String,
    
    /// Whether this instance should clean up on drop (only the first instance should)
    should_cleanup: bool,
    
    /// CRITICAL: Test execution lock to prevent concurrent test execution
    /// This must be held for the entire duration of the test to prevent race conditions
    #[allow(dead_code)]
    _test_lock: std::sync::MutexGuard<'static, ()>,
}

const MOCK_SERVER_CONFIG: &str = r#"# Test configuration with mock servers
proxy_enabled: true
proxy_host_ip: "localhost"  
proxy_port_number: 44340

suibase_api_port_number: 44399

# Override all links completely
links_overrides: true

links:
  - alias: "localnet"
    rpc: "http://localhost:9000"
    ws: "ws://localhost:9000"
    selectable: false  # IMPORTANT: Real server not selectable during tests
    monitored: true

  # Mock servers for testing
  - alias: "mock-0"
    rpc: "http://localhost:50001"
    selectable: true
    monitored: true

  - alias: "mock-1"
    rpc: "http://localhost:50002"
    selectable: true
    monitored: true

  - alias: "mock-2"
    rpc: "http://localhost:50003"
    selectable: true
    monitored: true

  - alias: "mock-3"
    rpc: "http://localhost:50004"
    selectable: true
    monitored: true

  - alias: "mock-4"
    rpc: "http://localhost:50005"
    selectable: true
    monitored: true
"#;

impl MockServerTestHarness {
    /// Create a new test harness with smart daemon lifecycle management
    /// Only restarts daemon when necessary (config change or daemon not running)
    /// 
    /// CRITICAL: This method acquires a global test execution lock to ensure
    /// only one test runs at a time, preventing race conditions with shared daemon state
    pub async fn new() -> Result<Self> {
        // CRITICAL: Acquire global test execution lock to prevent concurrent test execution
        // This lock is held for the entire duration of the test
        println!("🔒 Acquiring test execution lock to prevent race conditions...");
        let test_lock = TEST_EXECUTION_LOCK.lock().unwrap_or_else(|poisoned| {
            println!("⚠️  Test lock was poisoned - recovering...");
            poisoned.into_inner()
        });
        println!("✅ Test execution lock acquired - test can proceed safely");
        
        let yaml_path = PathBuf::from("/home/olet/suibase/workdirs/localnet/suibase.yaml");
        let api_base_url = "http://localhost:44399".to_string();
        let api_client = Client::new();
        
        let mut daemon_state = DAEMON_STATE.lock().unwrap_or_else(|poisoned| {
            println!("⚠️  Daemon state lock was poisoned - recovering...");
            poisoned.into_inner()
        });
        
        // Check if daemon is responsive
        let daemon_responsive = Self::is_daemon_responsive(&api_client, &api_base_url).await?;
        
        if daemon_responsive {
            // Daemon is running and responsive
            daemon_state.is_running = true;
            println!("✅ Daemon is running and responsive - no restart needed");
            
            // Always ensure test config is in place, but don't restart for config changes
            // This allows tests to run with the current daemon state
            if !yaml_path.exists() {
                println!("📝 Writing test config");
                std::fs::write(&yaml_path, MOCK_SERVER_CONFIG)?;
                
                // Give daemon a moment to pick up the config change  
                tokio::time::sleep(Duration::from_millis(500)).await;
                println!("✅ Test config written - daemon should auto-reload");
            } else {
                println!("📝 Test config already exists - ensuring it's current");
                
                // Rewrite the config to ensure it's exactly what we want
                std::fs::write(&yaml_path, MOCK_SERVER_CONFIG)?;
                tokio::time::sleep(Duration::from_millis(500)).await;
                println!("✅ Test config refreshed");
            }
        } else {
            // Daemon is not responsive - this is the only case where we restart
            println!("⚠️ Daemon is not responsive - restart required");
            daemon_state.last_restart_reason = Some("daemon not responsive".to_string());
            daemon_state.is_running = false;
            
            // Write the mock server configuration
            std::fs::write(&yaml_path, MOCK_SERVER_CONFIG)?;
            
            // Restart suibase-daemon
            let output = std::process::Command::new("/home/olet/suibase/scripts/dev/update-daemon")
                .output()?;
            
            if !output.status.success() {
                bail!("Failed to restart suibase-daemon: {}", String::from_utf8_lossy(&output.stderr));
            }
            
            // Wait for daemon to be ready
            Self::wait_for_daemon_ready(&api_client, &api_base_url).await?;
            
            // Update state
            daemon_state.is_running = true;
            daemon_state.last_restart_reason = None;
        }
        
        
        Ok(Self {
            api_client,
            yaml_path,
            api_base_url,
            should_cleanup: false, // Don't cleanup by default - leave config for debugging
            _test_lock: test_lock, // Hold lock for entire test duration
        })
    }
    
    /// Wait for daemon to be ready and responsive
    async fn wait_for_daemon_ready(client: &Client, api_url: &str) -> Result<()> {
        let start_time = Instant::now();
        loop {
            if start_time.elapsed() > Duration::from_secs(30) {
                bail!("Timeout waiting for suibase-daemon to start");
            }

            if Self::is_daemon_responsive(client, api_url).await? {
                break;
            }

            sleep(Duration::from_millis(500)).await;
        }
        Ok(())
    }
    
    /// Check if daemon is responsive
    async fn is_daemon_responsive(client: &Client, api_url: &str) -> Result<bool> {
        match client
            .post(api_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "getLinks",
                "params": {
                    "workdir": "localnet"
                },
                "id": 1
            }))
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
    

    /// Get detailed configuration state from debug API
    async fn get_config_debug_info(client: &Client, api_url: &str) -> Result<serde_json::Value> {
        let response = client
            .post(api_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "getLinks",
                "params": {
                    "workdir": "localnet",
                    "debug": true
                },
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to get config debug info: HTTP {}", response.status());
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(error) = result.get("error") {
            bail!("API error getting config debug info: {}", error);
        }

        Ok(result)
    }

    /// Notify the daemon about configuration file changes to accelerate processing
    async fn notify_config_change(client: &Client, api_url: &str, config_path: &str) -> Result<()> {
        let response = client
            .post(api_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "fsChange",
                "params": {
                    "path": config_path
                },
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to notify config change: HTTP {}", response.status());
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(error) = result.get("error") {
            bail!("API error notifying config change: {}", error);
        }

        println!("📢 Notified daemon about configuration file change");
        Ok(())
    }

    /// Parse debug output to extract link configuration parameters
    fn parse_link_config_from_debug(debug_str: &str, alias: &str) -> Option<serde_json::Value> {
        // Look for pattern like: "alias": Link { alias: "mock-0", selectable: true, monitored: true, rpc: Some("http://localhost:50001"), metrics: None, ws: None, priority: 255, max_per_secs: None, max_per_min: None }
        // We need to extract the max_per_secs and max_per_min values
        // The configuration we want is in the AdminController section, not the InputPort section
        
        // Find the AdminController section first
        let admin_section_start = if let Some(pos) = debug_str.find("AdminController:") {
            pos
        } else {
            // Fallback to searching the entire string if AdminController section is not found
            0
        };
        
        let search_section = &debug_str[admin_section_start..];
        
        if let Some(start_pos) = search_section.find(&format!("\"{}\": Link {{", alias)) {
            // Find the matching closing brace
            let slice = &search_section[start_pos..];
            let mut brace_count = 0;
            let mut end_pos = 0;
            
            for (i, ch) in slice.char_indices() {
                if ch == '{' {
                    brace_count += 1;
                } else if ch == '}' {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_pos = i;
                        break;
                    }
                }
            }
            
            if end_pos > 0 {
                let link_config = &slice[..=end_pos];
                
                // Extract max_per_secs value
                let max_per_secs = if let Some(secs_start) = link_config.find("max_per_secs: ") {
                    let secs_slice = &link_config[secs_start + "max_per_secs: ".len()..];
                    if secs_slice.starts_with("Some(") {
                        // Extract the number
                        if let Some(end) = secs_slice.find(')') {
                            if let Ok(value) = secs_slice[5..end].parse::<u64>() {
                                serde_json::Value::Number(serde_json::Number::from(value))
                            } else {
                                serde_json::Value::Null
                            }
                        } else {
                            serde_json::Value::Null
                        }
                    } else {
                        serde_json::Value::Null
                    }
                } else {
                    serde_json::Value::Null
                };
                
                // Extract max_per_min value
                let max_per_min = if let Some(min_start) = link_config.find("max_per_min: ") {
                    let min_slice = &link_config[min_start + "max_per_min: ".len()..];
                    if min_slice.starts_with("Some(") {
                        // Extract the number
                        if let Some(end) = min_slice.find(')') {
                            if let Ok(value) = min_slice[5..end].parse::<u64>() {
                                serde_json::Value::Number(serde_json::Number::from(value))
                            } else {
                                serde_json::Value::Null
                            }
                        } else {
                            serde_json::Value::Null
                        }
                    } else {
                        serde_json::Value::Null
                    }
                } else {
                    serde_json::Value::Null
                };
                
                // Extract selectable value
                let selectable = if let Some(sel_start) = link_config.find("selectable: ") {
                    let sel_slice = &link_config[sel_start + "selectable: ".len()..];
                    if sel_slice.starts_with("true") {
                        serde_json::Value::Bool(true)
                    } else if sel_slice.starts_with("false") {
                        serde_json::Value::Bool(false)
                    } else {
                        serde_json::Value::Null
                    }
                } else {
                    serde_json::Value::Null
                };
                
                return Some(json!({
                    "alias": alias,
                    "selectable": selectable,
                    "max_per_secs": max_per_secs,
                    "max_per_min": max_per_min
                }));
            }
        }
        
        None
    }

    /// Wait for configuration to match expectations (content-based verification)
    async fn wait_for_config_content_match(client: &Client, api_url: &str, expected_configs: &[serde_json::Value], timeout: Duration) -> Result<()> {
        let start = Instant::now();
        
        loop {
            if start.elapsed() > timeout {
                println!("🕐 Timeout waiting for configuration content to match expectations");
                println!("   - Expected configs: {}", serde_json::to_string_pretty(expected_configs)?);
                println!("   - Timeout duration: {:?}", timeout);
                println!("   - Elapsed time: {:?}", start.elapsed());
                bail!("Timeout waiting for daemon configuration to match expectations");
            }
            
            match Self::get_config_debug_info(client, api_url).await {
                Ok(debug_response) => {
                    if let Some(debug_str) = debug_response.get("result").and_then(|r| r.get("debug")).and_then(|d| d.as_str()) {
                        let mut all_match = true;
                        
                        for expected_config in expected_configs {
                            if let Some(alias) = expected_config.get("alias").and_then(|a| a.as_str()) {
                                if let Some(actual_config) = Self::parse_link_config_from_debug(debug_str, alias) {
                                    // Compare specific fields
                                    if let Some(expected_selectable) = expected_config.get("selectable") {
                                        if actual_config.get("selectable") != Some(expected_selectable) {
                                            all_match = false;
                                            break;
                                        }
                                    }
                                    
                                    if let Some(expected_max_per_secs) = expected_config.get("max_per_secs") {
                                        if actual_config.get("max_per_secs") != Some(expected_max_per_secs) {
                                            all_match = false;
                                            break;
                                        }
                                    }
                                    
                                    if let Some(expected_max_per_min) = expected_config.get("max_per_min") {
                                        if actual_config.get("max_per_min") != Some(expected_max_per_min) {
                                            all_match = false;
                                            break;
                                        }
                                    }
                                } else {
                                    println!("❌ Could not find configuration for {} in debug output", alias);
                                    all_match = false;
                                    break;
                                }
                            }
                        }
                        
                        if all_match {
                            return Ok(());
                        }
                    } else {
                        println!("⚠️ No debug information found in API response");
                    }
                },
                Err(e) => {
                    println!("⚠️ Failed to get config debug info: {}", e);
                }
            }
            
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Configure a specific mock server's behavior
    pub async fn configure_mock_server(&self, alias: &str, behavior: MockServerBehavior) -> Result<()> {
        let response = self.api_client
            .post(&self.api_base_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "mockServerControl",
                "params": {
                    "alias": alias,
                    "action": "set_behavior",
                    "behavior": behavior
                },
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to configure mock server {}: HTTP {}", alias, response.status());
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(error) = result.get("error") {
            bail!("API error configuring mock server {}: {}", alias, error);
        }

        Ok(())
    }

    /// Get statistics for a specific mock server
    pub async fn get_mock_server_stats(&self, alias: &str, reset_after: bool) -> Result<MockServerStatsResponse> {
        // IMPORTANT: Use reset_after=true to get real stats since read-only implementation is not complete
        let actual_reset_after = true; // Force reset_after to true to get real stats via AdminController
        
        let response = self.api_client
            .post(&self.api_base_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "mockServerStats",
                "params": {
                    "alias": alias,
                    "reset_after": actual_reset_after
                },
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to get mock server stats for {}: HTTP {}", alias, response.status());
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(error) = result.get("error") {
            bail!("API error getting mock server stats for {}: {}", alias, error);
        }

        let stats = result.get("result")
            .ok_or_else(|| anyhow!("Missing result in response"))?;
        
        Ok(serde_json::from_value(stats.clone())?)
    }

    /// Control multiple mock servers at once
    pub async fn control_servers_batch(&self, requests: Vec<MockServerControlRequest>) -> Result<()> {
        let response = self.api_client
            .post(&self.api_base_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "mockServerBatch",
                "params": {
                    "servers": requests
                },
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to execute batch control: HTTP {}", response.status());
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(error) = result.get("error") {
            bail!("API error in batch control: {}", error);
        }

        Ok(())
    }

    /// Get links debug information for a specific workdir
    pub async fn get_links_debug(&self, workdir: &str) -> Result<LinksResponse> {
        let response = self.api_client
            .post(&self.api_base_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "getLinks", 
                "params": {
                    "workdir": workdir,
                    "debug": true
                },
                "id": 1
            }))
            .send()
            .await?;
            
        if !response.status().is_success() {
            bail!("Failed to get links debug info: HTTP {}", response.status());
        }
        
        let result: serde_json::Value = response.json().await?;
        if let Some(error) = result.get("error") {
            bail!("API error getting links debug info: {}", error);
        }
        
        let links_response: LinksResponse = serde_json::from_value(result["result"].clone())?;
        Ok(links_response)
    }

    /// Get links/statistics from the proxy server
    pub async fn get_statistics(&self, workdir: &str) -> Result<LinksResponse> {
        let response = self.api_client
            .post(&self.api_base_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "getLinks",
                "params": {
                    "workdir": workdir,
                    "data": true,
                    "links": true
                },
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to get statistics: HTTP {}", response.status());
        }

        let result: serde_json::Value = response.json().await?;
        
        if let Some(error) = result.get("error") {
            bail!("API error getting statistics: {}", error);
        }

        let stats = result.get("result")
            .ok_or_else(|| anyhow!("Missing result in response"))?;
        
        serde_json::from_value(stats.clone())
            .map_err(|e| anyhow!("Failed to deserialize API response: {}", e))
    }

    /// Send a JSON-RPC request through the proxy server
    pub async fn send_rpc_request(&self, method: &str) -> Result<reqwest::Response> {
        let proxy_url = "http://localhost:44340";
        
        let response = self.api_client
            .post(proxy_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": [],
                "id": 1
            }))
            .send()
            .await?;

        Ok(response)
    }

    /// Send multiple JSON-RPC requests rapidly
    pub async fn send_rpc_burst(&self, count: usize, method: &str) -> Result<Vec<reqwest::Response>> {
        let mut tasks = Vec::new();
        
        for _ in 0..count {
            let client = self.api_client.clone();
            let method = method.to_string();
            
            let task = tokio::spawn(async move {
                let proxy_url = "http://localhost:44340";
                client
                    .post(proxy_url)
                    .json(&json!({
                        "jsonrpc": "2.0",
                        "method": method,
                        "params": [],
                        "id": 1
                    }))
                    .send()
                    .await
            });
            
            tasks.push(task);
        }

        let mut responses = Vec::new();
        for task in tasks {
            responses.push(task.await??);
        }

        Ok(responses)
    }

    /// Modify configuration and wait for it to be applied using content-based verification
    pub async fn modify_config_and_wait<F>(&self, modifier: F) -> Result<()>
    where
        F: FnOnce(&mut serde_yaml::Value),
    {
        // Start by waiting for any previous config changes to be fully processed
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        println!("🔍 Starting config modification with content-based verification...");
        
        // Read and modify config
        let config_content = std::fs::read_to_string(&self.yaml_path)?;
        let mut config: serde_yaml::Value = serde_yaml::from_str(&config_content)?;
        modifier(&mut config);
        
        // Extract expected configuration parameters for verification
        let expected_configs = Self::extract_expected_configs_from_yaml(&config)?;
        
        // Write modified config
        let modified_content = serde_yaml::to_string(&config)?;
        std::fs::write(&self.yaml_path, modified_content)?;
        println!("📝 Config file written - notifying daemon...");
        
        // Notify daemon about the configuration change to accelerate processing
        Self::notify_config_change(&self.api_client, &self.api_base_url, &self.yaml_path.to_string_lossy()).await?;
        
        // Wait for daemon to apply the changes using content verification
        Self::wait_for_config_content_match(&self.api_client, &self.api_base_url, &expected_configs, Duration::from_secs(10)).await?;
        
        Ok(())
    }

    /// Extract expected configuration parameters from YAML for verification
    fn extract_expected_configs_from_yaml(config: &serde_yaml::Value) -> Result<Vec<serde_json::Value>> {
        let mut expected_configs = Vec::new();
        
        if let Some(links) = config.get("links").and_then(|l| l.as_sequence()) {
            for link in links {
                if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                    let mut config_obj = json!({
                        "alias": alias
                    });
                    
                    // Extract selectable flag
                    if let Some(selectable) = link.get("selectable").and_then(|s| s.as_bool()) {
                        config_obj["selectable"] = json!(selectable);
                    }
                    
                    // Extract rate limiting parameters
                    if let Some(max_per_secs) = link.get("max_per_secs").and_then(|s| s.as_u64()) {
                        config_obj["max_per_secs"] = json!(max_per_secs);
                    } else {
                        config_obj["max_per_secs"] = json!(null);
                    }
                    
                    if let Some(max_per_min) = link.get("max_per_min").and_then(|s| s.as_u64()) {
                        config_obj["max_per_min"] = json!(max_per_min);
                    } else {
                        config_obj["max_per_min"] = json!(null);
                    }
                    
                    expected_configs.push(config_obj);
                }
            }
        }
        
        Ok(expected_configs)
    }


    /// Reset configuration file to baseline (clean state with no rate limits)
    pub async fn reset_to_baseline_config(&self) -> Result<()> {
        println!("🔄 Checking if baseline configuration reset is needed...");
        
        // First check if the current configuration already matches baseline
        if Self::config_matches_baseline(&self.yaml_path).await? {
            println!("✅ Configuration already matches baseline - no reset needed");
            // Still validate via API to be sure
            Self::validate_baseline_config(&self.api_client, &self.api_base_url).await?;
            return Ok(());
        }
        
        println!("🔄 Configuration differs from baseline - performing reset...");
        
        // Write baseline config
        std::fs::write(&self.yaml_path, MOCK_SERVER_CONFIG)?;
        println!("📝 Baseline config written - notifying daemon...");
        
        // Notify daemon about the configuration change to accelerate processing
        Self::notify_config_change(&self.api_client, &self.api_base_url, &self.yaml_path.to_string_lossy()).await?;
        
        // Parse baseline config to extract expected parameters for verification
        let baseline_config: serde_yaml::Value = serde_yaml::from_str(MOCK_SERVER_CONFIG)?;
        let expected_baseline_configs = Self::extract_expected_configs_from_yaml(&baseline_config)?;
        
        // Wait for daemon to apply the baseline config using content verification (shorter timeout now that we notify)
        Self::wait_for_config_content_match(&self.api_client, &self.api_base_url, &expected_baseline_configs, Duration::from_secs(10)).await?;
        
        // Now validate that the baseline configuration was properly applied via traditional API validation
        Self::validate_baseline_config(&self.api_client, &self.api_base_url).await?;
        
        println!("✅ Configuration reset to baseline completed and validated");
        Ok(())
    }

    /// Check if the current configuration file matches the baseline
    async fn config_matches_baseline(config_path: &std::path::Path) -> Result<bool> {
        if !config_path.exists() {
            return Ok(false);
        }
        
        let current_content = std::fs::read_to_string(config_path)?;
        let current_config: serde_yaml::Value = serde_yaml::from_str(&current_content)?;
        let baseline_config: serde_yaml::Value = serde_yaml::from_str(MOCK_SERVER_CONFIG)?;
        
        // Compare the parsed YAML structures
        Ok(current_config == baseline_config)
    }

    /// Validate that the baseline configuration is properly applied via API and file content
    async fn validate_baseline_config(client: &Client, api_url: &str) -> Result<()> {
        println!("🔍 Validating baseline configuration via API...");
        
        // Get current links configuration via API
        let response = client
            .post(api_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "getLinks",
                "params": {
                    "workdir": "localnet"
                },
                "id": 1
            }))
            .send()
            .await?;

        let json_response: serde_json::Value = response.json().await?;
        
        if let Some(result) = json_response.get("result") {
            if let Some(links) = result.get("links").and_then(|l| l.as_array()) {
                println!("📋 Found {} links in API response", links.len());
                
                // Validate localnet server (should be selectable=false in baseline)
                let localnet = links.iter().find(|link| {
                    link.get("alias").and_then(|a| a.as_str()) == Some("localnet")
                });
                
                if let Some(localnet_link) = localnet {
                    let selectable = localnet_link.get("selectable").and_then(|s| s.as_bool()).unwrap_or(true);
                    if selectable {
                        bail!("❌ Validation failed: localnet should be selectable=false in baseline config, but API shows selectable={}", selectable);
                    }
                    println!("✅ localnet correctly configured as selectable=false");
                } else {
                    bail!("❌ Validation failed: localnet server not found in API response");
                }
                
                // Validate mock servers (should exist and be selectable=true)
                let expected_mock_servers = ["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"];
                let mut found_mock_count = 0;
                
                for mock_alias in expected_mock_servers {
                    let mock_server = links.iter().find(|link| {
                        link.get("alias").and_then(|a| a.as_str()) == Some(mock_alias)
                    });
                    
                    if let Some(mock_link) = mock_server {
                        let selectable = mock_link.get("selectable").and_then(|s| s.as_bool()).unwrap_or(false);
                        if !selectable {
                            bail!("❌ Validation failed: {} should be selectable=true, but API shows selectable={}", mock_alias, selectable);
                        }
                        
                        println!("✅ {} correctly configured as selectable=true", mock_alias);
                        found_mock_count += 1;
                    } else {
                        bail!("❌ Validation failed: {} server not found in API response", mock_alias);
                    }
                }
                
                if found_mock_count != expected_mock_servers.len() {
                    bail!("❌ Validation failed: Expected {} mock servers, found {}", expected_mock_servers.len(), found_mock_count);
                }
                
                println!("✅ API validation passed - all servers present with correct selectability");
            } else {
                bail!("❌ Validation failed: No links found in API response");
            }
        } else {
            bail!("❌ Validation failed: No result found in API response");
        }
        
        // Also validate that the configuration file matches baseline (no rate limiting)
        Self::validate_baseline_config_file().await?;
        
        Ok(())
    }

    /// Validate that the configuration file content matches baseline (no rate limiting parameters)
    async fn validate_baseline_config_file() -> Result<()> {
        println!("🔍 Validating configuration file content...");
        
        let config_path = std::path::Path::new("/home/olet/suibase/workdirs/localnet/suibase.yaml");
        let config_content = std::fs::read_to_string(config_path)?;
        let config: serde_yaml::Value = serde_yaml::from_str(&config_content)?;
        
        if let Some(links) = config.get("links").and_then(|l| l.as_sequence()) {
            for link in links {
                if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                    if alias.starts_with("mock-") {
                        // Check that no rate limiting parameters are present
                        if link.get("max_per_secs").is_some() {
                            bail!("❌ Config validation failed: {} has max_per_secs parameter, should be removed in baseline", alias);
                        }
                        if link.get("max_per_min").is_some() {
                            bail!("❌ Config validation failed: {} has max_per_min parameter, should be removed in baseline", alias);
                        }
                        println!("✅ {} has no rate limiting parameters (as expected in baseline)", alias);
                    }
                }
            }
        } else {
            bail!("❌ Config validation failed: No links section found in configuration file");
        }
        
        println!("✅ Configuration file validation passed - no rate limiting parameters found");
        Ok(())
    }

    /// Minimal cleanup - just reset mock server behaviors to defaults
    /// Config is ALWAYS left in place for debugging
    pub async fn cleanup(&mut self) -> Result<()> {
        if self.should_cleanup {
            println!("🧹 Resetting mock server behaviors to defaults (keeping config for debugging)");
            // Just reset mock servers to default behavior, leave config for debugging
            let _ = reset_all_mock_servers(self).await;
        }
        Ok(())
    }
    
}

// Simplified Drop - no cleanup to preserve debugging state
impl Drop for MockServerTestHarness {
    fn drop(&mut self) {
        // Intentionally do nothing - preserve config for debugging
        // Use cleanup() or force_cleanup() methods for explicit cleanup
    }
}

/// Helper functions for common test scenarios

/// Reset all mock servers to default behavior
pub async fn reset_all_mock_servers(harness: &MockServerTestHarness) -> Result<()> {
    let default_behavior = MockServerBehavior::default();
    
    let requests = vec![
        MockServerControlRequest {
            alias: "mock-0".to_string(),
            behavior: default_behavior.clone(),
        },
        MockServerControlRequest {
            alias: "mock-1".to_string(),
            behavior: default_behavior.clone(),
        },
        MockServerControlRequest {
            alias: "mock-2".to_string(),
            behavior: default_behavior.clone(),
        },
        MockServerControlRequest {
            alias: "mock-3".to_string(),
            behavior: default_behavior.clone(),
        },
        MockServerControlRequest {
            alias: "mock-4".to_string(),
            behavior: default_behavior,
        },
    ];

    harness.control_servers_batch(requests).await?;
    Ok(())
}

/// Create a mock server behavior with failure rate
pub fn failing_behavior(failure_rate: f64) -> MockServerBehavior {
    MockServerBehavior {
        failure_rate,
        latency_ms: 0,
        http_status: 200,
        error_type: None,
        response_body: None,
    }
}

/// Create a mock server behavior with latency
pub fn slow_behavior(latency_ms: u32) -> MockServerBehavior {
    MockServerBehavior {
        failure_rate: 0.0,
        latency_ms,
        http_status: 200,
        error_type: None,
        response_body: None,
    }
}

/// Create a mock server behavior that returns specific JSON-RPC errors
pub fn error_response_behavior(error_response: serde_json::Value) -> MockServerBehavior {
    MockServerBehavior {
        failure_rate: 0.0,
        latency_ms: 0,
        http_status: 200,
        error_type: None,
        response_body: Some(error_response),
    }
}