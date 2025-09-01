use std::path::PathBuf;
use std::sync::Arc;

use common::{basic_types::*, log_safe_err, shared_types::{get_workdir_paths, WORKDIR_IDX_TESTNET, WORKDIR_IDX_MAINNET}};

use tokio::time::Instant;

use crate::shared_types::GlobalsWorkdirConfigMT;

use anyhow::{anyhow, Result};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

// Design
//
// The WalrusMonitor monitors the status of walrus-upload-relay processes for each workdir.
// It is a lightweight thread that:
//
// 1. Monitors if walrus-upload-relay processes are running as expected
// 2. Checks local connectivity by calling /v1/tip-config endpoint
// 3. Maintains status.yaml files for each workdir
// 4. Status values: DISABLED, INITIALIZING, OK, DOWN
//
// The actual process lifecycle (start/stop) is handled by bash scripts,
// this monitor only tracks status and maintains the status.yaml files.
//
// Status.yaml location: ~/suibase/workdirs/{testnet,mainnet}/walrus-relay/status.yaml

pub struct WalrusMonMsg {
    event_id: WalrusMonEvents,
    timestamp: EpochTimestamp,
    workdir_idx: WorkdirIdx,
    // Simple parameters for request reporting
    error_message: Option<String>,
}

impl WalrusMonMsg {
    pub fn new() -> Self {
        Self { 
            event_id: 0,
            timestamp: Instant::now(),
            workdir_idx: WORKDIR_IDX_TESTNET, // Default fallback
            error_message: None,
        }
    }
    
    pub fn new_with_event(event_id: WalrusMonEvents, workdir_idx: WorkdirIdx) -> Self {
        Self {
            event_id,
            timestamp: Instant::now(),
            workdir_idx,
            error_message: None,
        }
    }
    
    pub fn new_failure(error: String, workdir_idx: WorkdirIdx) -> Self {
        Self {
            event_id: EVENT_REPORT_REQ_FAILED,
            timestamp: Instant::now(),
            workdir_idx,
            error_message: Some(error),
        }
    }
}

// Events ID.
// See GenericChannelID for guidelines to set these values.
pub type WalrusMonEvents = u8;
pub const EVENT_REPORT_REQ_SUCCESS: u8 = 134; // Report a successful walrus relay request
pub const EVENT_REPORT_REQ_FAILED: u8 = 135;  // Report a failed walrus relay request

impl std::fmt::Debug for WalrusMonMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WalrusMonMsg {{ event_id: {}, timestamp: {:?}, error: {:?} }}", 
               self.event_id, self.timestamp, self.error_message)
    }
}

pub type WalrusMonTx = tokio::sync::mpsc::Sender<WalrusMonMsg>;
pub type WalrusMonRx = tokio::sync::mpsc::Receiver<WalrusMonMsg>;

// Public interface for Phase 4 HTTP proxy integration
// This allows the proxy server to report request statistics to WalrusMonitor
pub struct WalrusStatsReporter<'a> {
    tx_channel: &'a WalrusMonTx,
    workdir_idx: WorkdirIdx,
}

impl<'a> WalrusStatsReporter<'a> {
    pub fn new(tx_channel: &'a WalrusMonTx, workdir_idx: WorkdirIdx) -> Self {
        Self { tx_channel, workdir_idx }
    }
    
    // Report a successful request processed by the HTTP proxy
    pub async fn report_success(&self) -> Result<()> {
        WalrusMonitor::send_request_success(self.tx_channel, self.workdir_idx).await
    }
    
    // Report a failed request processed by the HTTP proxy
    pub async fn report_failure(&self, error: String) -> Result<()> {
        WalrusMonitor::send_request_failure(self.tx_channel, error, self.workdir_idx).await
    }
    
    // For future use: report success with additional context (workdir, latency, etc.)
    pub async fn report_success_with_context(&self, _workdir: &str, _latency_ms: Option<u64>) -> Result<()> {
        // For now, just report basic success
        // In the future, we can extend the message structure to include workdir context
        self.report_success().await
    }
    
    // For future use: report failure with additional context
    pub async fn report_failure_with_context(&self, _workdir: &str, error: String, _status_code: Option<u16>) -> Result<()> {
        // For now, just report basic failure
        // In the future, we can extend the message structure to include workdir context
        self.report_failure(error).await
    }
}

// Public API methods for accessing walrus stats (similar to JSON-RPC network stats)
impl WalrusMonitor {
    // Get testnet walrus relay statistics
    pub async fn get_testnet_stats(&self) -> WalrusStats {
        self.globals_testnet_walrus_stats.read().await.clone()
    }
    
    // Get mainnet walrus relay statistics  
    pub async fn get_mainnet_stats(&self) -> WalrusStats {
        self.globals_mainnet_walrus_stats.read().await.clone()
    }
    
    // Reset statistics (for testing or maintenance)
    pub async fn reset_testnet_stats(&self) {
        self.globals_testnet_walrus_stats.write().await.reset();
    }
    
    pub async fn reset_mainnet_stats(&self) {
        self.globals_mainnet_walrus_stats.write().await.reset();
    }
    
    // Get combined stats for all workdirs
    pub async fn get_combined_stats(&self) -> WalrusStats {
        let testnet_stats = self.globals_testnet_walrus_stats.read().await;
        let mainnet_stats = self.globals_mainnet_walrus_stats.read().await;
        
        let mut combined = WalrusStats::new();
        combined.total_requests = testnet_stats.total_requests() + mainnet_stats.total_requests();
        combined.successful_requests = testnet_stats.successful_requests() + mainnet_stats.successful_requests();
        combined.failed_requests = testnet_stats.failed_requests() + mainnet_stats.failed_requests();
        combined.last_activity = [
            testnet_stats.last_activity(),
            mainnet_stats.last_activity()
        ].into_iter()
         .flatten()
         .max();
        combined
    }
}

// Simple statistics tracking for Walrus relay requests
#[derive(Debug, Clone)]
pub struct WalrusStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    last_activity: Option<EpochTimestamp>,
    last_error: Option<String>,
}

impl WalrusStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            last_activity: None,
            last_error: None,
        }
    }
    
    pub fn reset(&mut self) {
        self.total_requests = 0;
        self.successful_requests = 0;
        self.failed_requests = 0;
        self.last_activity = None;
        self.last_error = None;
    }
    
    pub fn record_success(&mut self, timestamp: EpochTimestamp) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_activity = Some(timestamp);
        self.last_error = None; // Clear error on success
    }
    
    pub fn record_failure(&mut self, timestamp: EpochTimestamp, error: Option<String>) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_activity = Some(timestamp);
        if let Some(err) = error {
            self.last_error = Some(err);
        }
    }
    
    // Getters for stats
    pub fn total_requests(&self) -> u64 { self.total_requests }
    pub fn successful_requests(&self) -> u64 { self.successful_requests }
    pub fn failed_requests(&self) -> u64 { self.failed_requests }
    pub fn last_activity(&self) -> Option<EpochTimestamp> { self.last_activity }
    pub fn last_error(&self) -> Option<&String> { self.last_error.as_ref() }
    
    // Convenience methods for API
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64) / (self.total_requests as f64)
        }
    }
    
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.failed_requests as f64) / (self.total_requests as f64)
        }
    }
    
    pub fn has_activity(&self) -> bool {
        self.last_activity.is_some()
    }
    
    pub fn has_errors(&self) -> bool {
        self.failed_requests > 0 || self.last_error.is_some()
    }
}

pub struct WalrusMonitor {
    globals_testnet_config: GlobalsWorkdirConfigMT,
    globals_mainnet_config: GlobalsWorkdirConfigMT,
    globals_testnet_walrus_stats: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,
    globals_mainnet_walrus_stats: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,
    walrusmon_rx: WalrusMonRx,
    http_client: reqwest::Client,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct WalrusRelayStatus {
    status: String,           // "DISABLED", "INITIALIZING", "OK", "DOWN"
    local_connectivity: String, // "OK", "ERROR", "UNKNOWN"
    last_check: String,       // ISO 8601 timestamp
    error_message: Option<String>, // Error details if any
    proxy_port: Option<u16>,  // Actual listening port detected from process
    local_port: Option<u16>,  // Local walrus relay port
    // Note: Request statistics are NOT stored in status.yaml
    // They are served via API like JSON-RPC network stats
}

impl WalrusRelayStatus {
    fn new() -> Self {
        Self {
            status: "INITIALIZING".to_string(),
            local_connectivity: "UNKNOWN".to_string(),
            last_check: chrono::Utc::now().to_rfc3339(),
            error_message: None,
            proxy_port: None,
            local_port: None,
        }
    }

    fn set_disabled(&mut self) {
        self.status = "DISABLED".to_string();
        self.local_connectivity = "UNKNOWN".to_string();
        self.last_check = chrono::Utc::now().to_rfc3339();
        self.error_message = None;
        self.proxy_port = None;
        self.local_port = None;
    }

    fn set_down(&mut self, error: Option<String>) {
        self.status = "DOWN".to_string();
        self.local_connectivity = "ERROR".to_string();
        self.last_check = chrono::Utc::now().to_rfc3339();
        self.error_message = error;
        // Keep port info as it might still be useful for debugging
    }

    fn set_ok(&mut self, proxy_port: Option<u16>, local_port: Option<u16>) {
        self.status = "OK".to_string();
        self.local_connectivity = "OK".to_string();
        self.last_check = chrono::Utc::now().to_rfc3339();
        self.error_message = None;
        self.proxy_port = proxy_port;
        self.local_port = local_port;
    }

}

impl WalrusMonitor {
    pub fn new(
        globals_testnet_config: GlobalsWorkdirConfigMT,
        globals_mainnet_config: GlobalsWorkdirConfigMT,
        globals_testnet_walrus_stats: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,
        globals_mainnet_walrus_stats: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,
        walrusmon_rx: WalrusMonRx,
    ) -> Self {
        Self {
            globals_testnet_config,
            globals_mainnet_config,
            globals_testnet_walrus_stats,
            globals_mainnet_walrus_stats,
            walrusmon_rx,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn send_event_audit(tx_channel: &WalrusMonTx) -> Result<()> {
        let mut msg = WalrusMonMsg::new();
        msg.event_id = EVENT_AUDIT;
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    pub async fn send_event_update(tx_channel: &WalrusMonTx) -> Result<()> {
        let mut msg = WalrusMonMsg::new();
        msg.event_id = EVENT_UPDATE;
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }
    
    // Report a successful request (for Phase 4 HTTP proxy integration)
    pub async fn send_request_success(tx_channel: &WalrusMonTx, workdir_idx: WorkdirIdx) -> Result<()> {
        let msg = WalrusMonMsg::new_with_event(EVENT_REPORT_REQ_SUCCESS, workdir_idx);
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("Failed to send success report: {}", e);
            anyhow!("failed {}", e)
        })
    }
    
    // Report a failed request (for Phase 4 HTTP proxy integration)
    pub async fn send_request_failure(tx_channel: &WalrusMonTx, error: String, workdir_idx: WorkdirIdx) -> Result<()> {
        let msg = WalrusMonMsg::new_failure(error, workdir_idx);
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("Failed to send failure report: {}", e);
            anyhow!("failed {}", e)
        })
    }


    // Check backend connectivity by calling the health endpoint
    async fn check_local_connectivity(&self, local_port: u16) -> (String, Option<String>) {
        let url = format!("http://localhost:{}/v1/tip-config", local_port);
        
        match self.http_client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    ("OK".to_string(), None)
                } else {
                    let error_msg = format!("HTTP {}", response.status());
                    ("ERROR".to_string(), Some(error_msg))
                }
            }
            Err(e) => {
                let error_msg = if e.is_timeout() {
                    "Timeout connecting to local walrus relay".to_string()
                } else {
                    format!("Connection error: {}", e)
                };
                ("ERROR".to_string(), Some(error_msg))
            }
        }
    }

    // Find PID of walrus-upload-relay process
    async fn find_walrus_relay_pid() -> Option<u32> {
        // Try pgrep first (cross-platform)
        if let Ok(output) = tokio::process::Command::new("pgrep")
            .args(&["-f", "walrus-upload-relay"])
            .output()
            .await {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(pid) = output_str.trim().parse::<u32>() {
                return Some(pid);
            }
        }

        // Fallback: try ps with grep
        if let Ok(output) = tokio::process::Command::new("ps")
            .args(&["aux"])
            .output()
            .await {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("walrus-upload-relay") && !line.contains("grep") {
                    // Parse ps output to get PID (second field)
                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields.len() >= 2 {
                        if let Ok(pid) = fields[1].parse::<u32>() {
                            return Some(pid);
                        }
                    }
                }
            }
        }

        None
    }

    // Detect listening port from process ID using cross-platform approach
    fn detect_listening_ports(pid: u32) -> (Option<u16>, Option<u16>) {
        // Try lsof first (cross-platform and more reliable)
        if let Ok(output) = std::process::Command::new("lsof")
            .args(&["-p", &pid.to_string(), "-i", "-P", "-n"])
            .output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut proxy_port = None;
            let mut local_port = None;
            
            for line in output_str.lines() {
                if line.contains("LISTEN") {
                    // Parse lsof output: walrus-up 12345 user 6u IPv4 0x... 0t0 TCP *:45852 (LISTEN)
                    if let Some(tcp_part) = line.split_whitespace().find(|s| s.contains("TCP")) {
                        if let Some(port_str) = tcp_part.split(':').last() {
                            if let Ok(port) = port_str.parse::<u16>() {
                                // Heuristic: proxy ports are usually 458xx, local ports 458xx
                                if port >= 45850 && port <= 45860 {
                                    proxy_port = Some(port);
                                } else if port >= 45800 && port <= 45810 {
                                    local_port = Some(port);
                                }
                            }
                        }
                    }
                }
            }
            
            return (proxy_port, local_port);
        }

        // Fallback for Linux: try netstat
        #[cfg(target_os = "linux")]
        if let Ok(output) = std::process::Command::new("netstat")
            .args(&["-lnp"])
            .output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut proxy_port = None;
            let mut local_port = None;
            
            for line in output_str.lines() {
                if line.contains(&pid.to_string()) {
                    // Parse netstat output: tcp 0 0 0.0.0.0:45852 0.0.0.0:* LISTEN 12345/walrus-upload
                    if let Some(addr_part) = line.split_whitespace().nth(3) {
                        if let Some(port_str) = addr_part.split(':').last() {
                            if let Ok(port) = port_str.parse::<u16>() {
                                if port >= 45850 && port <= 45860 {
                                    proxy_port = Some(port);
                                } else if port >= 45800 && port <= 45810 {
                                    local_port = Some(port);
                                }
                            }
                        }
                    }
                }
            }
            
            return (proxy_port, local_port);
        }

        // If both fail, return None
        (None, None)
    }

    // Write status.yaml file for a workdir
    async fn write_status_yaml(&self, workdir_path: &PathBuf, status: &WalrusRelayStatus) -> Result<()> {
        let status_dir = workdir_path.join("walrus-relay");
        let status_file = status_dir.join("status.yaml");

        // Ensure directory exists
        if let Err(e) = tokio::fs::create_dir_all(&status_dir).await {
            return Err(anyhow!("Failed to create status directory {}: {}", status_dir.display(), e));
        }

        // Serialize status to YAML
        let yaml_content = serde_yaml::to_string(status).map_err(|e| {
            anyhow!("Failed to serialize status to YAML: {}", e)
        })?;

        // Write to file
        tokio::fs::write(&status_file, yaml_content).await.map_err(|e| {
            anyhow!("Failed to write status file {}: {}", status_file.display(), e)
        })?;

        Ok(())
    }

    // Monitor walrus relay status for a specific workdir
    // Note: stats parameter kept for future API integration but not used in status.yaml
    async fn monitor_workdir(&self, workdir_name: &str, workdir_config: &GlobalsWorkdirConfigMT, _stats: &WalrusStats) -> WalrusRelayStatus {
        let mut status = WalrusRelayStatus::new();
        log::debug!("Monitoring Walrus relay status for {}", workdir_name);

        let (enabled, local_port, proxy_port) = {
            let config_guard = workdir_config.read().await;
            let user_config = &config_guard.user_config;
            
            (
                user_config.is_walrus_relay_enabled(),
                user_config.walrus_relay_local_port(),
                user_config.walrus_relay_proxy_port(),
            )
        };

        if !enabled || local_port == 0 {
            status.set_disabled();
            return status;
        }

        // Status starts as INITIALIZING, now test the API endpoint
        let (connectivity, error) = self.check_local_connectivity(local_port).await;
        status.local_connectivity = connectivity.clone();
        status.error_message = error;

        if connectivity == "OK" {
            // Try to detect actual running ports from process
            let detected_pid = Self::find_walrus_relay_pid().await;
            let (detected_proxy_port, detected_local_port) = if let Some(pid) = detected_pid {
                Self::detect_listening_ports(pid)
            } else {
                (None, None)
            };
            
            // Use detected ports if available, otherwise fall back to config
            let final_proxy_port = detected_proxy_port.or(Some(proxy_port));
            let final_local_port = detected_local_port.or(Some(local_port));
            
            status.set_ok(final_proxy_port, final_local_port);
        } else {
            status.set_down(status.error_message.clone());
        }
        
        // Note: Stats are NOT included in status.yaml - they will be served via API

        status
    }

    async fn audit(&mut self) {
        log::debug!("WalrusMonitor audit starting");

        // Monitor testnet
        let testnet_stats = self.globals_testnet_walrus_stats.read().await.clone();
        let testnet_status = self.monitor_workdir("testnet", &self.globals_testnet_config, &testnet_stats).await;
        
        // Global testnet status is updated via YAML file writes
        
        let testnet_path = {
            let config_guard = self.globals_testnet_config.read().await;
            get_workdir_paths(config_guard.workdir_idx).workdir_root_path().to_path_buf()
        };
        
        if let Err(e) = self.write_status_yaml(&testnet_path, &testnet_status).await {
            log_safe_err!(format!("Failed to write testnet status.yaml: {}", e));
        }

        // Monitor mainnet
        let mainnet_stats = self.globals_mainnet_walrus_stats.read().await.clone();
        let mainnet_status = self.monitor_workdir("mainnet", &self.globals_mainnet_config, &mainnet_stats).await;
        
        // Global mainnet status is updated via YAML file writes
        
        let mainnet_path = {
            let config_guard = self.globals_mainnet_config.read().await;
            get_workdir_paths(config_guard.workdir_idx).workdir_root_path().to_path_buf()
        };
        
        if let Err(e) = self.write_status_yaml(&mainnet_path, &mainnet_status).await {
            log_safe_err!(format!("Failed to write mainnet status.yaml: {}", e));
        }

        log::debug!("WalrusMonitor audit completed");
    }

    async fn process_msg(&mut self, msg: WalrusMonMsg) {
        match msg.event_id {
            EVENT_AUDIT => {
                self.audit().await;
            }
            EVENT_UPDATE => {
                self.audit().await; // Immediate status update on config changes
            }
            EVENT_REPORT_REQ_SUCCESS => {
                log::debug!("Recording successful walrus relay request for workdir {}", msg.workdir_idx);
                
                // Update only the specific workdir's statistics
                match msg.workdir_idx {
                    WORKDIR_IDX_TESTNET => {
                        let mut testnet_stats = self.globals_testnet_walrus_stats.write().await;
                        testnet_stats.record_success(msg.timestamp);
                    }
                    WORKDIR_IDX_MAINNET => {
                        let mut mainnet_stats = self.globals_mainnet_walrus_stats.write().await;
                        mainnet_stats.record_success(msg.timestamp);
                    }
                    _ => {
                        log::debug!("Unknown workdir_idx {} for success report, ignoring", msg.workdir_idx);
                    }
                }
            }
            EVENT_REPORT_REQ_FAILED => {
                log::debug!("WalrusMonitor: Recording failed walrus relay request for workdir {}: {:?}", msg.workdir_idx, msg.error_message);
                
                // Update only the specific workdir's statistics
                match msg.workdir_idx {
                    WORKDIR_IDX_TESTNET => {
                        let mut testnet_stats = self.globals_testnet_walrus_stats.write().await;
                        testnet_stats.record_failure(msg.timestamp, msg.error_message.clone());
                        log::debug!("WalrusMonitor: Updated testnet stats - total: {}, failed: {}", testnet_stats.total_requests(), testnet_stats.failed_requests());
                    }
                    WORKDIR_IDX_MAINNET => {
                        let mut mainnet_stats = self.globals_mainnet_walrus_stats.write().await;
                        mainnet_stats.record_failure(msg.timestamp, msg.error_message.clone());
                        log::debug!("WalrusMonitor: Updated mainnet stats - total: {}, failed: {}", mainnet_stats.total_requests(), mainnet_stats.failed_requests());
                    }
                    _ => {
                        log::debug!("Unknown workdir_idx {} for failure report, ignoring", msg.workdir_idx);
                    }
                }
            }
            _ => {
                log::debug!("process_msg unexpected event id {}", msg.event_id);
            }
        }
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            let cur_msg = self.walrusmon_rx.recv().await;
            if cur_msg.is_none() || subsys.is_shutdown_requested() {
                // Channel closed or shutdown requested.
                return;
            }
            self.process_msg(cur_msg.unwrap()).await;
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("WalrusMonitor started");

        // The loop to handle all incoming messages.
        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("WalrusMonitor normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("WalrusMonitor normal thread exit (1)");
                Ok(())
            }
        }
    }
}