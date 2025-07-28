// Integration tests for rate limiting configuration parsing

use common::shared_types::workdirs::WorkdirUserConfig;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_sample_rate_limit_configuration() {
    let yaml_content = r#"
# Sample suibase.yaml demonstrating rate limiting configuration
proxy_enabled: true
proxy_port_number: 44399

links:
  # High-traffic public RPC with conservative rate limit
  - alias: "mainnet_public"
    rpc: "https://fullnode.mainnet.sui.io:443"
    max_per_secs: 50
    priority: 100

  # Testnet RPC with moderate rate limit
  - alias: "testnet_public"
    rpc: "https://fullnode.testnet.sui.io:443"
    max_per_secs: 100
    priority: 90

  # Local development node - higher rate limit
  - alias: "localnet"
    rpc: "http://localhost:9000"
    max_per_secs: 500
    priority: 10

  # Premium RPC service - very high rate limit
  - alias: "premium_rpc"
    rpc: "https://premium-sui-rpc.example.com:443"
    max_per_secs: 1000
    priority: 5

  # Emergency fallback - no rate limit
  - alias: "emergency_fallback"
    rpc: "https://backup-sui-rpc.example.com:443"
    priority: 200
    # max_per_secs not specified = unlimited

  # Rate limited to prevent abuse
  - alias: "shared_development"
    rpc: "https://shared-dev-sui.example.com:443"
    max_per_secs: 10
    priority: 150
"#;

    // Create temporary file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(yaml_content.as_bytes()).expect("Failed to write temp file");
    let temp_path = temp_file.path().to_str().expect("Failed to get temp path");

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);
    
    assert!(result.is_ok(), "Failed to parse configuration: {:?}", result.err());
    
    // Verify proxy settings
    assert_eq!(config.is_proxy_enabled(), true);
    assert_eq!(config.proxy_port_number(), 44399);
    
    let links = config.links();
    assert_eq!(links.len(), 6);

    // Test mainnet_public with rate limit
    let mainnet_link = links.get("mainnet_public").unwrap();
    assert_eq!(mainnet_link.rpc, Some("https://fullnode.mainnet.sui.io:443".to_string()));
    assert_eq!(mainnet_link.max_per_secs, Some(50));
    assert_eq!(mainnet_link.priority, 100);

    // Test testnet_public with different rate limit
    let testnet_link = links.get("testnet_public").unwrap();
    assert_eq!(testnet_link.rpc, Some("https://fullnode.testnet.sui.io:443".to_string()));
    assert_eq!(testnet_link.max_per_secs, Some(100));
    assert_eq!(testnet_link.priority, 90);

    // Test localnet with high rate limit
    let localnet_link = links.get("localnet").unwrap();
    assert_eq!(localnet_link.rpc, Some("http://localhost:9000".to_string()));
    assert_eq!(localnet_link.max_per_secs, Some(500));
    assert_eq!(localnet_link.priority, 10);

    // Test premium_rpc with very high rate limit
    let premium_link = links.get("premium_rpc").unwrap();
    assert_eq!(premium_link.rpc, Some("https://premium-sui-rpc.example.com:443".to_string()));
    assert_eq!(premium_link.max_per_secs, Some(1000));
    assert_eq!(premium_link.priority, 5);

    // Test emergency_fallback with no rate limit
    let emergency_link = links.get("emergency_fallback").unwrap();
    assert_eq!(emergency_link.rpc, Some("https://backup-sui-rpc.example.com:443".to_string()));
    assert_eq!(emergency_link.max_per_secs, None); // No rate limit
    assert_eq!(emergency_link.priority, 200);

    // Test shared_development with low rate limit
    let shared_link = links.get("shared_development").unwrap();
    assert_eq!(shared_link.rpc, Some("https://shared-dev-sui.example.com:443".to_string()));
    assert_eq!(shared_link.max_per_secs, Some(10));
    assert_eq!(shared_link.priority, 150);
}

#[test]
fn test_realistic_mixed_configuration() {
    let yaml_content = r#"
proxy_enabled: true

links:
  # Production servers with conservative limits
  - alias: "sui_mainnet_1"
    rpc: "https://fullnode.mainnet.sui.io:443"
    max_per_secs: 30
    priority: 50

  - alias: "sui_mainnet_2" 
    rpc: "https://rpc-mainnet.suiscan.xyz:443"
    max_per_secs: 40
    priority: 60

  # Development/testing servers with higher limits
  - alias: "sui_testnet"
    rpc: "https://fullnode.testnet.sui.io:443"
    max_per_secs: 150
    priority: 20

  - alias: "local_node"
    rpc: "http://127.0.0.1:9000"
    max_per_secs: 2000
    priority: 10

  # Third-party services
  - alias: "alchemy_sui"
    rpc: "https://sui-mainnet.g.alchemy.com/v2/demo"
    max_per_secs: 200
    priority: 30

  # Backup without rate limit
  - alias: "backup_node"
    rpc: "https://backup.example.com:443"
    priority: 100
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);
    
    assert!(result.is_ok());
    
    let links = config.links();
    assert_eq!(links.len(), 6);

    // Verify rate limits are parsed correctly
    assert_eq!(links.get("sui_mainnet_1").unwrap().max_per_secs, Some(30));
    assert_eq!(links.get("sui_mainnet_2").unwrap().max_per_secs, Some(40));
    assert_eq!(links.get("sui_testnet").unwrap().max_per_secs, Some(150));
    assert_eq!(links.get("local_node").unwrap().max_per_secs, Some(2000));
    assert_eq!(links.get("alchemy_sui").unwrap().max_per_secs, Some(200));
    assert_eq!(links.get("backup_node").unwrap().max_per_secs, None); // No limit

    // Verify priorities are parsed correctly
    assert_eq!(links.get("local_node").unwrap().priority, 10);      // Highest priority
    assert_eq!(links.get("sui_testnet").unwrap().priority, 20);
    assert_eq!(links.get("alchemy_sui").unwrap().priority, 30);
    assert_eq!(links.get("sui_mainnet_1").unwrap().priority, 50);
    assert_eq!(links.get("sui_mainnet_2").unwrap().priority, 60);
    assert_eq!(links.get("backup_node").unwrap().priority, 100);    // Lowest priority
}

#[test]
fn test_edge_cases_in_configuration() {
    let yaml_content = r#"
links:
  # Zero rate limit (should block all requests)
  - alias: "blocked_server"
    rpc: "https://blocked.example.com:443"
    max_per_secs: 0

  # Maximum u32 value
  - alias: "max_rate_server"
    rpc: "https://fast.example.com:443"
    max_per_secs: 4294967295

  # Very low rate limit
  - alias: "slow_server"
    rpc: "https://slow.example.com:443"
    max_per_secs: 1

  # Regular server for comparison
  - alias: "normal_server"
    rpc: "https://normal.example.com:443"
    max_per_secs: 100
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);
    
    assert!(result.is_ok());
    
    let links = config.links();
    assert_eq!(links.len(), 4);

    // Test edge case values
    assert_eq!(links.get("blocked_server").unwrap().max_per_secs, Some(0));
    assert_eq!(links.get("max_rate_server").unwrap().max_per_secs, Some(u32::MAX));
    assert_eq!(links.get("slow_server").unwrap().max_per_secs, Some(1));
    assert_eq!(links.get("normal_server").unwrap().max_per_secs, Some(100));
}