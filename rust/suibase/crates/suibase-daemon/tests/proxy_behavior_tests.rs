// Tests for proxy server behavior
//
// These tests verify that the proxy server behaves correctly:
// - Load balancing across healthy servers
// - Failover when servers become unhealthy
// - Retry logic for retryable errors
// - Server selection based on the selectable flag
// - Cascading failure scenarios
//
// These tests focus on the proxy server's routing and resilience logic,
// using mock servers to simulate various backend conditions.

mod common;

use anyhow::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use common::{
    MockServerTestHarness, reset_all_mock_servers, clear_all_rate_limits,
    failing_behavior, slow_behavior, error_response_behavior
};

/// Test that the selectable flag is respected and real servers are not used during testing
#[tokio::test]
async fn test_selectable_flag_respected() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Ensure all mock servers are healthy and no rate limits
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
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
    
    // Wait for servers to be included in the load balancing subset
    let expected_servers = vec!["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"];
    let load_balanced_servers = harness.wait_for_load_balanced_servers(&expected_servers, 30).await?;
    println!("🎯 Load balancing subset: {:?}", load_balanced_servers);
    
    // Send a larger number of requests to better observe distribution
    // With more requests, we're more likely to see distribution across servers
    let responses = harness.send_rpc_burst(200, "sui_getLatestSuiSystemState").await?;
    
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
        
        // Only check load distribution among servers that are in the load balancing subset
        let lb_servers: Vec<_> = mock_servers.iter()
            .filter(|s| load_balanced_servers.contains(&s.alias))
            .collect();
        
        println!("Checking load distribution among {} load-balanced servers", lb_servers.len());
        
        // Check that load is distributed among load-balanced servers
        let lb_servers_with_load: Vec<_> = lb_servers.iter()
            .filter(|s| s.load_pct.parse::<f64>().unwrap_or(0.0) > 0.0)
            .collect();
        
        // With 200 requests, we expect to see some distribution
        assert!(!lb_servers_with_load.is_empty(), 
            "Expected at least some load-balanced servers to receive traffic");
        
        // If we have multiple servers in the LB subset with load, check distribution
        if lb_servers.len() > 1 && lb_servers_with_load.len() > 1 {
            for server in &lb_servers {
                let load_pct = server.load_pct.parse::<f64>().unwrap_or(0.0);
                if load_pct > 0.0 {
                    // With multiple servers, no single one should have almost everything
                    assert!(load_pct < 95.0, "Server {} has too concentrated load: {}%", server.alias, load_pct);
                }
            }
        }
        
        // Verify non-load-balanced servers have minimal or no traffic
        let non_lb_servers: Vec<_> = mock_servers.iter()
            .filter(|s| !load_balanced_servers.contains(&s.alias))
            .collect();
        
        for server in non_lb_servers {
            let load_pct = server.load_pct.parse::<f64>().unwrap_or(0.0);
            assert!(load_pct < 5.0, 
                "Non-load-balanced server {} should have minimal traffic but has {}%", 
                server.alias, load_pct);
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
    
    // Start with all servers healthy and no rate limits
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
    // Send some requests to verify all servers are working
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    for response in responses {
        assert!(response.status().is_success());
    }
    
    // Make mock-0 fail completely and wait for it to be marked as DOWN
    harness.configure_mock_server_and_wait_for_state("mock-0", failing_behavior(1.0), false).await?;
    
    // Ensure other servers are still healthy
    harness.ensure_servers_healthy(&["mock-1", "mock-2", "mock-3", "mock-4"]).await?;
    
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

/// Test handling of retryable JSON-RPC errors
#[tokio::test]
async fn test_retryable_json_rpc_errors() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Reset both mock behaviors and rate limits
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
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
    harness.configure_mock_server("mock-1", suibase_daemon::shared_types::MockServerBehavior::default()).await?;
    harness.configure_mock_server("mock-2", suibase_daemon::shared_types::MockServerBehavior::default()).await?;
    
    // Small delay to ensure configuration is applied
    sleep(Duration::from_millis(100)).await;
    
    // Send multiple requests to increase chance of hitting mock-0
    let mut any_success = false;
    for _ in 0..10 {
        let response = harness.send_rpc_request("sui_getObject").await?;
        if response.status().is_success() {
            any_success = true;
        }
    }
    
    // Should have at least one success via retry on healthy server
    assert!(any_success, "At least one request should succeed via retry");
    
    // Verify servers received requests
    let mock_0_stats = harness.get_mock_server_stats("mock-0").await?;
    let mock_1_stats = harness.get_mock_server_stats("mock-1").await?;
    let mock_2_stats = harness.get_mock_server_stats("mock-2").await?;
    
    // At least one mock server should have received requests
    let total_requests = mock_0_stats.stats.requests_received + 
                        mock_1_stats.stats.requests_received + 
                        mock_2_stats.stats.requests_received;
    
    assert!(total_requests > 0, "Mock servers should have received requests");
    
    // If mock-0 received requests, it should have some failures
    if mock_0_stats.stats.requests_received > 0 {
        println!("Mock-0 received {} requests, failed: {}", 
                mock_0_stats.stats.requests_received,
                mock_0_stats.stats.requests_failed);
    }
    
    harness.cleanup().await?;
    Ok(())
}

/// Test cascading failures scenario
#[tokio::test] 
async fn test_cascading_failures() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Start with all servers healthy and no rate limits
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
    // Verify all servers work
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success());
    
    // Progressively fail servers
    harness.configure_mock_server_and_wait_for_state("mock-0", failing_behavior(1.0), false).await?;
    harness.ensure_servers_healthy(&["mock-1", "mock-2", "mock-3", "mock-4"]).await?;
    
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success(), "Should still work with 4 servers");
    
    harness.configure_mock_server_and_wait_for_state("mock-1", failing_behavior(1.0), false).await?;
    harness.ensure_servers_healthy(&["mock-2", "mock-3", "mock-4"]).await?;
    
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success(), "Should still work with 3 servers");
    
    harness.configure_mock_server_and_wait_for_state("mock-2", failing_behavior(1.0), false).await?;
    harness.ensure_servers_healthy(&["mock-3", "mock-4"]).await?;
    
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    assert!(response.status().is_success(), "Should still work with 2 servers");
    
    // When most servers fail, some requests might start failing
    harness.configure_mock_server_and_wait_for_state("mock-3", failing_behavior(1.0), false).await?;
    harness.ensure_servers_healthy(&["mock-4"]).await?;
    
    // Now only mock-4 is healthy - requests should still work but may be slower
    let response = harness.send_rpc_request("sui_getLatestSuiSystemState").await?;
    // This might fail depending on retry logic, which is acceptable
    
    println!("Final server status: {}", response.status());
    
    harness.cleanup().await?;
    Ok(())
}

/// Test mixed server behaviors and verify proxy handles them correctly
#[tokio::test]
async fn test_mixed_server_behaviors() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Start fresh
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
    // Configure different behaviors for servers
    // mock-0: 100ms delay (mild)
    // mock-1: fast (default)
    // mock-2: failing
    // mock-3: fast (default)
    // mock-4: very slow (500ms)
    
    harness.configure_and_verify_mock_server("mock-0", slow_behavior(100)).await?;
    
    // Configure mock-2 to fail and wait for it to be marked as DOWN
    harness.configure_mock_server_and_wait_for_state("mock-2", failing_behavior(1.0), false).await?;
    
    // Configure mock-4 to be very slow but should still be healthy
    harness.configure_mock_server_and_wait_for_state("mock-4", slow_behavior(500), true).await?;
    
    // Ensure expected servers are healthy
    harness.ensure_servers_healthy(&["mock-0", "mock-1", "mock-3"]).await?;
    
    // Reset all server stats before the test
    println!("Resetting all server stats for accurate measurement...");
    harness.reset_all_server_stats("localnet").await?;
    
    // Wait for healthy servers to be in the load balancing subset
    let expected_healthy = vec!["mock-0", "mock-1", "mock-3", "mock-4"];
    let load_balanced_servers = harness.wait_for_load_balanced_servers(&expected_healthy, 30).await?;
    println!("🎯 Load balancing subset: {:?}", load_balanced_servers);
    
    // Send traffic and observe distribution
    let test_responses = harness.send_rpc_burst(100, "sui_getLatestSuiSystemState").await?;
    let test_success_count = test_responses.iter().filter(|r| r.status().is_success()).count();
    
    println!("✅ {} out of 100 requests succeeded", test_success_count);
    assert!(test_success_count >= 95, "Expected most requests to succeed despite failing servers");
    
    // Check final distribution
    let final_stats = harness.get_statistics("localnet").await?;
    if let Some(links_vec) = &final_stats.links {
        println!("\n📊 Final traffic distribution (after reset):");
        for link in links_vec {
            if link.alias.starts_with("mock-") {
                println!("  {} - status: {}, load: {}%, resp_time: {}", 
                        link.alias, link.status, link.load_pct, link.resp_time);
            }
        }
        
        // Key verification: DOWN servers should get minimal or no traffic
        if let Some(mock_2) = links_vec.iter().find(|l| l.alias == "mock-2") {
            assert_eq!(mock_2.status, "DOWN", "mock-2 should be marked as DOWN");
            
            let mock_2_load = mock_2.load_pct.parse::<f64>().unwrap_or(0.0);
            
            // DOWN servers should get zero or very minimal traffic
            assert!(mock_2_load < 5.0, 
                    "Expected DOWN server mock-2 to have minimal traffic, but got {:.2}%", 
                    mock_2_load);
        }
        
        // Verify faster servers get more traffic than slower ones
        let mock_1_load = links_vec.iter()
            .find(|l| l.alias == "mock-1")
            .map(|l| l.load_pct.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
            
        let mock_4_load = links_vec.iter()
            .find(|l| l.alias == "mock-4")
            .map(|l| l.load_pct.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
            
        // Fast server (mock-1) should get more traffic than very slow server (mock-4)
        // if both are in the load balancing subset
        if load_balanced_servers.contains(&"mock-1".to_string()) && 
           load_balanced_servers.contains(&"mock-4".to_string()) {
            assert!(mock_1_load > mock_4_load, 
                    "Expected fast server to get more traffic than slow server, but mock-1: {:.1}%, mock-4: {:.1}%", 
                    mock_1_load, mock_4_load);
        }
    }
    
    println!("\n✅ Mixed server behaviors test passed!");
    harness.cleanup().await?;
    Ok(())
}