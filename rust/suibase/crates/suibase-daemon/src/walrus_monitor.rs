use std::path::PathBuf;

use common::{basic_types::*, log_safe_err, shared_types::get_workdir_paths};

use crate::shared_types::{GlobalsWorkdirConfigMT, GlobalsWorkdirStatusMT};

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
}

impl WalrusMonMsg {
    pub fn new() -> Self {
        Self { event_id: 0 }
    }
}

// Events ID.
// See GenericChannelID for guidelines to set these values.
pub type WalrusMonEvents = u8;

impl std::fmt::Debug for WalrusMonMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WalrusMonMsg {{ event_id: {} }}", self.event_id)
    }
}

pub type WalrusMonTx = tokio::sync::mpsc::Sender<WalrusMonMsg>;
pub type WalrusMonRx = tokio::sync::mpsc::Receiver<WalrusMonMsg>;

pub struct WalrusMonitor {
    globals_testnet_config: GlobalsWorkdirConfigMT,
    globals_mainnet_config: GlobalsWorkdirConfigMT,
    globals_testnet_status: GlobalsWorkdirStatusMT,
    globals_mainnet_status: GlobalsWorkdirStatusMT,
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
        globals_testnet_status: GlobalsWorkdirStatusMT,
        globals_mainnet_status: GlobalsWorkdirStatusMT,
        walrusmon_rx: WalrusMonRx,
    ) -> Self {
        Self {
            globals_testnet_config,
            globals_mainnet_config,
            globals_testnet_status,
            globals_mainnet_status,
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
    async fn monitor_workdir(&self, workdir_name: &str, workdir_config: &GlobalsWorkdirConfigMT) -> WalrusRelayStatus {
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

        status
    }

    async fn audit(&mut self) {
        log::debug!("WalrusMonitor audit starting");

        // Monitor testnet
        let testnet_status = self.monitor_workdir("testnet", &self.globals_testnet_config).await;
        
        // Update global testnet status
        {
            let mut _status_guard = self.globals_testnet_status.write().await;
            // TODO: Update the status fields as needed
        }
        
        let testnet_path = {
            let config_guard = self.globals_testnet_config.read().await;
            get_workdir_paths(config_guard.workdir_idx).workdir_root_path().to_path_buf()
        };
        
        if let Err(e) = self.write_status_yaml(&testnet_path, &testnet_status).await {
            log_safe_err!(format!("Failed to write testnet status.yaml: {}", e));
        }

        // Monitor mainnet
        let mainnet_status = self.monitor_workdir("mainnet", &self.globals_mainnet_config).await;
        
        // Update global mainnet status
        {
            let mut _status_guard = self.globals_mainnet_status.write().await;
            // TODO: Update the status fields as needed
        }
        
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