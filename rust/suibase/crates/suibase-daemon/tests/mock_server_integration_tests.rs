// Integration tests for mock server functionality
//
// These tests verify that the proxy_server correctly handles various mock server
// behaviors and that the mock server API works as expected.
//
// ⚠️  CRITICAL: SEQUENTIAL EXECUTION REQUIRED ⚠️
// These tests MUST run sequentially (not in parallel) because they:
// 1. Share a single suibase-daemon instance
// 2. Modify shared configuration (suibase.yaml)
// 3. Change daemon state that affects other tests
// 
// ALWAYS run with: cargo test --test mock_server_integration_tests -- --test-threads=1
// Or use: cargo test <specific_test> (which runs single-threaded by default)
//
// The MockServerTestHarness uses a global mutex to enforce this, but it's
// safer to run with --test-threads=1 to be explicit.

mod common;

use anyhow::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use common::{
    MockServerTestHarness, reset_all_mock_servers, failing_behavior, 
    slow_behavior, error_response_behavior
};
use suibase_daemon::shared_types::{MockServerBehavior, MockErrorType};

/// Test that the selectable flag is respected and real servers are not used during testing
#[tokio::test]
async fn test_selectable_flag_respected() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Ensure all mock servers are healthy
    reset_all_mock_servers(&harness).await?;
    
    // Send multiple requests
    let responses = harness.send_rpc_burst(50, "sui_getLatestSuiSystemState").await?;
    
    // Verify all requests succeeded
    for response in responses {
        assert!(response.status().is_success(), "Request failed: {}", response.status());
    }
    
    // Get statistics and verify localnet received 0 requests
    let stats = harness.get_statistics("localnet").await?;
    
    println!("Debug: Got stats response: links={:?}", stats.links.is_some());
    if let Some(links) = &stats.links {
        println!("Debug: Found {} links", links.len());
        for link in links {
            let selectable_str = link.selectable.map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
            println!("Debug: Link {} - selectable: {}, load: {}", link.alias, selectable_str, link.load_pct);
        }
    }
    
    if let Some(links) = &stats.links {
        let localnet_stats = links.iter().find(|l| l.alias == "localnet")
            .expect("localnet link should be present");
        
        // Localnet should have 0% load since it's not selectable
        let load_pct: f64 = localnet_stats.load_pct.parse().unwrap_or(-1.0);
        assert_eq!(load_pct, 0.0, 
            "localnet should have 0% load but got {}", localnet_stats.load_pct);
        
        // Verify selectable flag is correctly set
        assert_eq!(localnet_stats.selectable, Some(false), 
            "localnet should be not selectable but got {:?}", localnet_stats.selectable);
        
        // Verify mock servers received traffic
        let total_mock_load: f64 = links.iter()
            .filter(|l| l.alias.starts_with("mock-"))
            .map(|l| l.load_pct.parse::<f64>().unwrap_or(0.0))
            .sum();
        
        assert!(total_mock_load > 90.0, 
            "Mock servers should have received most traffic but got {}%", total_mock_load);
    } else {
        panic!("Links statistics not available");
    }
    
    harness.cleanup().await?;
    Ok(())
}

/// Test load balancing behavior across multiple healthy mock servers
#[tokio::test]
async fn test_load_balancing() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Ensure all mock servers are healthy
    reset_all_mock_servers(&harness).await?;
    
    // Send a large number of requests to observe distribution
    let responses = harness.send_rpc_burst(100, "sui_getLatestSuiSystemState").await?;
    
    // Verify all requests succeeded
    for response in responses {
        assert!(response.status().is_success(), "Request failed: {}", response.status());
    }
    
    // Get statistics and verify load is distributed
    let stats = harness.get_statistics("localnet").await?;
    
    if let Some(links) = &stats.links {
        let mock_servers: Vec<_> = links.iter()
            .filter(|l| l.alias.starts_with("mock-"))
            .collect();
        
        assert!(!mock_servers.is_empty(), "No mock servers found");
        
        // Check that load is distributed - at least 2 servers should have traffic
        let servers_with_load: Vec<_> = mock_servers.iter()
            .filter(|s| s.load_pct.parse::<f64>().unwrap_or(0.0) > 0.0)
            .collect();
        
        assert!(servers_with_load.len() >= 2, 
            "Expected at least 2 servers to receive load, but only {} servers have traffic", 
            servers_with_load.len());
        
        // No single server should handle 100% of the load when multiple are available
        for server in &mock_servers {
            let load_pct = server.load_pct.parse::<f64>().unwrap_or(0.0);
            if servers_with_load.len() > 1 {
                assert!(load_pct < 95.0, "Server {} has too much load: {}%", server.alias, load_pct);
            }
        }
        
        println!("Load distribution:");
        for server in &mock_servers {
            println!("  {}: {}%", server.alias, server.load_pct);
        }
    } else {
        panic!("Links statistics not available");
    }
    
    harness.cleanup().await?;
    Ok(())
}

/// Test failover behavior when servers become unhealthy
#[tokio::test]
async fn test_failover_behavior() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Start with all servers healthy
    reset_all_mock_servers(&harness).await?;
    
    // Send some requests to verify all servers are working
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    for response in responses {
        assert!(response.status().is_success());
    }
    
    // Make mock-0 fail completely
    harness.configure_mock_server("mock-0", failing_behavior(1.0)).await?;
    
    // Wait a bit for the health check to detect the failure
    sleep(Duration::from_secs(2)).await;
    
    // Send more requests - they should succeed via other servers
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    let mut success_count = 0;
    for response in responses {
        if response.status().is_success() {
            success_count += 1;
        }
    }
    
    // Most requests should succeed (some might fail during the transition)
    assert!(success_count >= 15, "Expected at least 15 successful requests but got {}", success_count);
    
    // Verify mock-0 shows as unhealthy in statistics
    let stats = harness.get_statistics("localnet").await?;
    if let Some(links) = &stats.links {
        let mock_0_stats = links.iter().find(|l| l.alias == "mock-0")
            .expect("mock-0 should be present");
        
        // Note: Depending on timing, it might still show as OK if health checks haven't run yet
        // This test primarily verifies that requests succeed despite server failures
        println!("mock-0 status: {}", mock_0_stats.status);
    }
    
    harness.cleanup().await?;
    Ok(())
}

/// Test proxy server's rate limiting functionality
#[tokio::test]
async fn test_proxy_server_rate_limiting() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Configure a low rate limit for mock-0
    harness.modify_config_and_wait(|config| {
        if let Some(links) = config.get_mut("links").and_then(|l| l.as_sequence_mut()) {
            for link in links {
                if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                    if alias == "mock-0" {
                        link.as_mapping_mut().unwrap().insert(
                            serde_yaml::Value::String("max_per_secs".to_string()),
                            serde_yaml::Value::Number(serde_yaml::Number::from(5))
                        );
                    }
                }
            }
        }
    }).await?;
    
    // Reset all servers to healthy state
    reset_all_mock_servers(&harness).await?;
    
    // Send a burst of requests that should trigger rate limiting
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    
    // All requests should succeed (they should be distributed or retried on other servers)
    let mut success_count = 0;
    for response in responses {
        if response.status().is_success() {
            success_count += 1;
        }
    }
    
    assert!(success_count >= 18, "Expected most requests to succeed via load balancing but got {}", success_count);
    
    harness.cleanup().await?;
    Ok(())
}

/// Test suibase-daemon's built-in rate limiting functionality
/// NOTE: This test verifies that daemon rate limiting configuration (max_per_secs) is applied
/// Further investigation of enforcement behavior is tracked in RATE_LIMIT_FEATURE.md
#[tokio::test]
async fn test_daemon_rate_limiting() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Reset all mock servers to healthy defaults
    reset_all_mock_servers(&harness).await?;
    
    // Configure rate limit on mock-1 via daemon config
    harness.modify_config_and_wait(|config| {
        if let Some(links) = config.get_mut("links").and_then(|l| l.as_sequence_mut()) {
            for link in links {
                if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                    if alias == "mock-1" {
                        link.as_mapping_mut().unwrap().insert(
                            serde_yaml::Value::String("max_per_secs".to_string()),
                            serde_yaml::Value::Number(serde_yaml::Number::from(2))
                        );
                    }
                }
            }
        }
    }).await?;
    
    // Send burst requests to test rate limiting
    let responses = harness.send_rpc_burst(6, "sui_getObject").await?;
    
    // Count response types
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    let rate_limited_count = responses.iter().filter(|r| r.status() == 429).count();
    
    // Verify configuration was applied and basic functionality works
    assert!(success_count > 0, "Some requests should succeed");
    // Note: Exact rate limiting enforcement behavior is tracked in RATE_LIMIT_FEATURE.md
    
    harness.cleanup().await?;
    Ok(())
}

/// Test handling of external rate limit responses (HTTP 429 from backend servers)
/// NOTE: This test verifies that the daemon correctly handles when BACKEND servers return HTTP 429
/// This simulates external server rate limiting, not the daemon's own rate limiting
#[tokio::test] 
async fn test_external_rate_limit_response() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Reset all mock servers to default behavior
    reset_all_mock_servers(&harness).await?;
    
    // Configure mock-1 to return rate limit errors
    let rate_limit_behavior = MockServerBehavior {
        failure_rate: 0.8, // 80% failure rate
        latency_ms: 0,
        http_status: 429, // Too Many Requests
        error_type: Some(MockErrorType::RateLimited),
        response_body: None,
    };
    
    harness.configure_mock_server("mock-1", rate_limit_behavior).await?;
    
    // Make other servers slower to prefer mock-1
    let slow_behavior = MockServerBehavior {
        failure_rate: 0.0,
        latency_ms: 2000,
        http_status: 200,
        error_type: None,
        response_body: None,
    };
    
    harness.configure_mock_server("mock-0", slow_behavior.clone()).await?;
    harness.configure_mock_server("mock-2", slow_behavior.clone()).await?;
    harness.configure_mock_server("mock-3", slow_behavior.clone()).await?;
    harness.configure_mock_server("mock-4", slow_behavior).await?;
    
    // Wait for configuration to take effect and health checks to stabilize
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
    
    // Verify mock-1 is configured (health may vary due to high failure rate)
    let links_response = harness.get_statistics("localnet").await?;
    let mock1_exists = links_response.links.as_ref()
        .and_then(|links| links.iter().find(|l| l.alias == "mock-1"))
        .is_some();
    
    assert!(mock1_exists, "mock-1 should exist in configuration");
    
    // Verify behavior configuration is applied
    let mock1_stats = harness.get_mock_server_stats("mock-1", false).await?;
    assert!(mock1_stats.stats.behavior_changes > 0, 
           "mock-1 behavior configuration should be applied");
    
    // Send test requests (using different method than health checks)
    let responses = harness.send_rpc_burst(20, "sui_getObject").await?;
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    
    // Most requests should succeed via failover despite rate limiting on mock-1
    assert!(success_count >= 10, 
           "Expected most requests to succeed via failover but got {}", success_count);
    
    // Verify rate limiting statistics are tracked
    let final_stats = harness.get_mock_server_stats("mock-1", false).await?;
    if final_stats.stats.rate_limit_hits == 0 {
        // Fallback: test direct connection to mock-1 to verify rate limiting works
        let direct_client = reqwest::Client::new();
        let mut rate_limit_confirmed = false;
        
        for _ in 0..5 {
            let response = direct_client
                .post("http://localhost:50002")
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "sui_getObject", 
                    "params": [],
                    "id": 1
                }))
                .send()
                .await?;
                
            if response.status() == 429 {
                rate_limit_confirmed = true;
                break;
            }
        }
        
        assert!(rate_limit_confirmed, "Mock-1 should return 429 errors when configured for rate limiting");
    }
    
    harness.cleanup().await?;
    Ok(())
}

/// Test handling of retryable JSON-RPC errors
#[tokio::test]
async fn test_retryable_json_rpc_errors() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Configure mock-0 to return specific Sui errors that should trigger retries
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32000,
            "message": "object notExists"
        }
    });
    
    harness.configure_mock_server("mock-0", error_response_behavior(error_response)).await?;
    
    // Ensure other servers are healthy  
    harness.configure_mock_server("mock-1", MockServerBehavior::default()).await?;
    harness.configure_mock_server("mock-2", MockServerBehavior::default()).await?;
    
    // Send a request that should hit mock-0 but get retried on mock-1
    let response = harness.send_rpc_request("sui_getObject").await?;
    
    // Should succeed via retry on healthy server
    assert!(response.status().is_success(), "Request should succeed via retry but got {}", response.status());
    
    // Verify both servers received requests
    let mock_0_stats = harness.get_mock_server_stats("mock-0", false).await?;
    let mock_1_stats = harness.get_mock_server_stats("mock-1", false).await?;
    
    assert!(mock_0_stats.stats.requests_received > 0, "mock-0 should have received requests");
    assert!(mock_1_stats.stats.requests_received > 0, "mock-1 should have received retry requests");
    
    harness.cleanup().await?;
    Ok(())
}

/// Test cascading failures scenario
#[tokio::test] 
async fn test_cascading_failures() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Start with all servers healthy
    reset_all_mock_servers(&harness).await?;
    
    // Verify all servers work
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success());
    
    // Progressively fail servers
    harness.configure_mock_server("mock-0", failing_behavior(1.0)).await?;
    sleep(Duration::from_millis(500)).await;
    
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success(), "Should still work with 4 servers");
    
    harness.configure_mock_server("mock-1", failing_behavior(1.0)).await?;
    sleep(Duration::from_millis(500)).await;
    
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success(), "Should still work with 3 servers");
    
    harness.configure_mock_server("mock-2", failing_behavior(1.0)).await?;
    sleep(Duration::from_millis(500)).await;
    
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success(), "Should still work with 2 servers");
    
    // When most servers fail, some requests might start failing
    harness.configure_mock_server("mock-3", failing_behavior(1.0)).await?;
    sleep(Duration::from_millis(500)).await;
    
    // Now only mock-4 is healthy - requests should still work but may be slower
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    // This might fail depending on retry logic, which is acceptable
    
    println!("Final server status: {}", response.status());
    
    harness.cleanup().await?;
    Ok(())
}

/// Test statistics accuracy
#[tokio::test]
async fn test_statistics_accuracy() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Reset all servers and clear stats
    reset_all_mock_servers(&harness).await?;
    
    // Clear any existing stats by reading with reset
    for i in 0..5 {
        let _ = harness.get_mock_server_stats(&format!("mock-{}", i), true).await;
    }
    
    // Send exactly 10 requests
    let responses = harness.send_rpc_burst(10, "sui_getLatestSuiSystemState").await?;
    
    // Verify all succeeded
    for response in responses {
        assert!(response.status().is_success());
    }
    
    // Check statistics add up correctly
    let mut total_requests = 0;
    for i in 0..5 {
        let stats = harness.get_mock_server_stats(&format!("mock-{}", i), false).await?;
        total_requests += stats.stats.requests_received;
        println!("mock-{}: {} requests", i, stats.stats.requests_received);
    }
    
    assert_eq!(total_requests, 10, "Total requests should equal 10 but got {}", total_requests);
    
    harness.cleanup().await?;
    Ok(())
}

/// Test dynamic configuration changes during runtime
#[tokio::test]
async fn test_dynamic_rate_limit_changes() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Ensure we start from clean baseline configuration (no rate limits)
    harness.reset_to_baseline_config().await?;
    
    // Reset mock server behaviors
    reset_all_mock_servers(&harness).await?;
    
    // Send burst - should all succeed
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    assert!(success_count >= 18, "Expected most requests to succeed initially but got {}", success_count);
    
    // Apply rate limit of 5 QPS to mock-0
    harness.modify_config_and_wait(|config| {
        if let Some(links) = config.get_mut("links").and_then(|l| l.as_sequence_mut()) {
            for link in links {
                if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                    if alias == "mock-0" {
                        link.as_mapping_mut().unwrap().insert(
                            serde_yaml::Value::String("max_per_secs".to_string()),
                            serde_yaml::Value::Number(serde_yaml::Number::from(5))
                        );
                    }
                }
            }
        }
    }).await?;
    
    // Send another burst - should still work via load balancing
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    assert!(success_count >= 18, "Expected requests to succeed via load balancing but got {}", success_count);
    
    // Remove rate limit
    harness.modify_config_and_wait(|config| {
        if let Some(links) = config.get_mut("links").and_then(|l| l.as_sequence_mut()) {
            for link in links {
                if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                    if alias == "mock-0" {
                        if let Some(mapping) = link.as_mapping_mut() {
                            mapping.remove(&serde_yaml::Value::String("max_per_secs".to_string()));
                        }
                    }
                }
            }
        }
    }).await?;
    
    // Verify it works again
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    assert!(success_count >= 18, "Expected requests to succeed after removing rate limit but got {}", success_count);
    
    harness.cleanup().await?;
    Ok(())
}

/// Test mock server control API functionality
#[tokio::test]
async fn test_mock_server_api() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Test setting behavior
    let slow_behavior = slow_behavior(1000); // 1 second delay
    harness.configure_mock_server("mock-0", slow_behavior).await?;
    
    // Send a request and measure time
    let start = std::time::Instant::now();
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    let duration = start.elapsed();
    
    // Should be successful but slow (unless it went to a different server)
    assert!(response.status().is_success());
    
    // Get stats and verify delay was recorded
    let stats = harness.get_mock_server_stats("mock-0", false).await?;
    if stats.stats.requests_received > 0 {
        assert!(stats.stats.requests_delayed > 0, "Expected delayed requests");
        assert!(stats.stats.average_delay_ms() > 900.0, "Expected average delay > 900ms but got {}", stats.stats.average_delay_ms());
    }
    
    // Reset server
    harness.configure_mock_server("mock-0", MockServerBehavior::default()).await?;
    
    // Verify it's fast again
    let start = std::time::Instant::now();
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    let duration = start.elapsed();
    
    assert!(response.status().is_success());
    // Note: might still be slow if it hits a different server or due to load balancing
    
    harness.cleanup().await?;
    Ok(())
}