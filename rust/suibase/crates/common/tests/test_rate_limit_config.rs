// Integration tests for rate limiting configuration parsing

use common::shared_types::workdirs::WorkdirUserConfig;
use std::io::Write;
use tempfile::NamedTempFile;

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
    temp_file
        .write_all(yaml_content.as_bytes())
        .expect("Failed to write temp file");
    let temp_path = temp_file.path().to_str().expect("Failed to get temp path");

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);

    assert!(
        result.is_ok(),
        "Failed to parse configuration: {:?}",
        result.err()
    );

    // Verify proxy settings
    assert_eq!(config.is_proxy_enabled(), true);
    assert_eq!(config.proxy_port_number(), 44399);

    let links = config.links();
    assert_eq!(links.len(), 6);

    // Test mainnet_public with rate limit
    let mainnet_link = links.get("mainnet_public").unwrap();
    assert_eq!(
        mainnet_link.rpc,
        Some("https://fullnode.mainnet.sui.io:443".to_string())
    );
    assert_eq!(mainnet_link.max_per_secs, Some(50));
    assert_eq!(mainnet_link.priority, 100);

    // Test testnet_public with different rate limit
    let testnet_link = links.get("testnet_public").unwrap();
    assert_eq!(
        testnet_link.rpc,
        Some("https://fullnode.testnet.sui.io:443".to_string())
    );
    assert_eq!(testnet_link.max_per_secs, Some(100));
    assert_eq!(testnet_link.priority, 90);

    // Test localnet with high rate limit
    let localnet_link = links.get("localnet").unwrap();
    assert_eq!(localnet_link.rpc, Some("http://localhost:9000".to_string()));
    assert_eq!(localnet_link.max_per_secs, Some(500));
    assert_eq!(localnet_link.priority, 10);

    // Test premium_rpc with very high rate limit
    let premium_link = links.get("premium_rpc").unwrap();
    assert_eq!(
        premium_link.rpc,
        Some("https://premium-sui-rpc.example.com:443".to_string())
    );
    assert_eq!(premium_link.max_per_secs, Some(1000));
    assert_eq!(premium_link.priority, 5);

    // Test emergency_fallback with no rate limit
    let emergency_link = links.get("emergency_fallback").unwrap();
    assert_eq!(
        emergency_link.rpc,
        Some("https://backup-sui-rpc.example.com:443".to_string())
    );
    assert_eq!(emergency_link.max_per_secs, None); // No rate limit
    assert_eq!(emergency_link.priority, 200);

    // Test shared_development with low rate limit
    let shared_link = links.get("shared_development").unwrap();
    assert_eq!(
        shared_link.rpc,
        Some("https://shared-dev-sui.example.com:443".to_string())
    );
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
    assert_eq!(links.get("local_node").unwrap().priority, 10); // Highest priority
    assert_eq!(links.get("sui_testnet").unwrap().priority, 20);
    assert_eq!(links.get("alchemy_sui").unwrap().priority, 30);
    assert_eq!(links.get("sui_mainnet_1").unwrap().priority, 50);
    assert_eq!(links.get("sui_mainnet_2").unwrap().priority, 60);
    assert_eq!(links.get("backup_node").unwrap().priority, 100); // Lowest priority
}

#[test]
fn test_edge_cases_in_configuration() {
    let yaml_content = r#"
links:
  # Zero rate limit (unlimited by new semantics)
  - alias: "unlimited_server"
    rpc: "https://unlimited.example.com:443"
    max_per_secs: 0
    max_per_min: 0

  # Maximum valid values for bit fields
  - alias: "max_rate_server"
    rpc: "https://fast.example.com:443"
    max_per_secs: 32767   # Maximum for 15-bit field
    max_per_min: 262143   # Maximum for 18-bit field

  # Very low rate limit
  - alias: "slow_server"
    rpc: "https://slow.example.com:443"
    max_per_secs: 1
    max_per_min: 10

  # Regular server for comparison
  - alias: "normal_server"
    rpc: "https://normal.example.com:443"
    max_per_secs: 100
    max_per_min: 5000
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
    let unlimited = links.get("unlimited_server").unwrap();
    assert_eq!(unlimited.max_per_secs, Some(0));
    assert_eq!(unlimited.max_per_min, Some(0));

    let max_rate = links.get("max_rate_server").unwrap();
    assert_eq!(max_rate.max_per_secs, Some(32767));
    assert_eq!(max_rate.max_per_min, Some(262143));

    let slow = links.get("slow_server").unwrap();
    assert_eq!(slow.max_per_secs, Some(1));
    assert_eq!(slow.max_per_min, Some(10));

    let normal = links.get("normal_server").unwrap();
    assert_eq!(normal.max_per_secs, Some(100));
    assert_eq!(normal.max_per_min, Some(5000));
}

#[test]
fn test_dual_rate_limiting_configuration() {
    let yaml_content = r#"
proxy_enabled: true

links:
  # QPS only (unlimited QPM)
  - alias: "qps_only"
    rpc: "https://qps-only.example.com:443"
    max_per_secs: 50
    # max_per_min not specified = unlimited

  # QPM only (unlimited QPS)
  - alias: "qpm_only"
    rpc: "https://qpm-only.example.com:443"
    max_per_min: 1000
    # max_per_secs not specified = unlimited

  # Both limits specified
  - alias: "dual_limits"
    rpc: "https://dual-limits.example.com:443"
    max_per_secs: 20
    max_per_min: 800

  # Neither limit specified (unlimited)
  - alias: "unlimited"
    rpc: "https://unlimited.example.com:443"
    # Both limits unspecified = unlimited

  # Zero values (unlimited by new semantics)
  - alias: "zero_unlimited"
    rpc: "https://zero-unlimited.example.com:443"
    max_per_secs: 0
    max_per_min: 0

  # Mixed zero/nonzero
  - alias: "mixed_limits"
    rpc: "https://mixed.example.com:443"
    max_per_secs: 0    # Unlimited QPS
    max_per_min: 500   # Limited QPM
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);

    assert!(
        result.is_ok(),
        "Failed to parse dual rate limiting config: {:?}",
        result.err()
    );

    let links = config.links();
    assert_eq!(links.len(), 6);

    // QPS only
    let qps_only = links.get("qps_only").unwrap();
    assert_eq!(qps_only.max_per_secs, Some(50));
    assert_eq!(qps_only.max_per_min, None);

    // QPM only
    let qpm_only = links.get("qpm_only").unwrap();
    assert_eq!(qpm_only.max_per_secs, None);
    assert_eq!(qpm_only.max_per_min, Some(1000));

    // Both limits
    let dual = links.get("dual_limits").unwrap();
    assert_eq!(dual.max_per_secs, Some(20));
    assert_eq!(dual.max_per_min, Some(800));

    // Unlimited
    let unlimited = links.get("unlimited").unwrap();
    assert_eq!(unlimited.max_per_secs, None);
    assert_eq!(unlimited.max_per_min, None);

    // Zero unlimited
    let zero_unlimited = links.get("zero_unlimited").unwrap();
    assert_eq!(zero_unlimited.max_per_secs, Some(0));
    assert_eq!(zero_unlimited.max_per_min, Some(0));

    // Mixed limits
    let mixed = links.get("mixed_limits").unwrap();
    assert_eq!(mixed.max_per_secs, Some(0));
    assert_eq!(mixed.max_per_min, Some(500));
}

#[test]
fn test_configuration_with_invalid_yaml_syntax() {
    // Test completely malformed YAML
    let invalid_yaml = r#"
links:
  - alias: "test"
    rpc: "https://test.com"
    max_per_secs: [invalid: structure}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(invalid_yaml.as_bytes()).unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);

    // Should fail to parse malformed YAML syntax
    assert!(result.is_err());
}

#[test]
fn test_configuration_with_string_rate_limits() {
    // Test string values for numeric fields (YAML parser might accept these)
    let yaml_content = r#"
links:
  - alias: "string_test"
    rpc: "https://test.example.com:443"
    max_per_secs: "not_a_number"
    max_per_min: "also_not_a_number"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);

    // This should fail because strings can't be parsed as u32
    if result.is_ok() {
        // If parsing succeeded, the fields should be None (ignored)
        let links = config.links();
        let link = links.get("string_test").unwrap();
        assert_eq!(link.max_per_secs, None);
        assert_eq!(link.max_per_min, None);
    } else {
        // If parsing failed, that's also acceptable
        assert!(result.is_err());
    }
}

#[test]
fn test_large_rate_limits() {
    // Test very large values that exceed bit field limits
    let yaml_content = r#"
links:
  - alias: "too_large_qps"
    rpc: "https://test.example.com:443"
    max_per_secs: 50000   # Exceeds 32767 limit
    
  - alias: "too_large_qpm"  
    rpc: "https://test2.example.com:443"
    max_per_min: 300000   # Exceeds 262143 limit
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    let temp_path = temp_file.path().to_str().unwrap();

    let mut config = WorkdirUserConfig::new();
    let result = config.load_and_merge_from_file(temp_path);

    // Should successfully parse (validation happens at RateLimiter::new())
    assert!(result.is_ok());

    let links = config.links();
    let large_qps = links.get("too_large_qps").unwrap();
    let large_qpm = links.get("too_large_qpm").unwrap();

    assert_eq!(large_qps.max_per_secs, Some(50000));
    assert_eq!(large_qpm.max_per_min, Some(300000));
}
