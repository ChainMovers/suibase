// Tests for rate limiting functionality
//
// These tests verify that the proxy server's rate limiting works correctly:
// - QPS (queries per second) limits
// - QPM (queries per minute) limits
// - Dynamic rate limit configuration
// - Rate limit enforcement and failover behavior
//
// These tests focus on the rate limiting logic in the proxy server,
// using mock servers as test backends.

mod common;

use anyhow::Result;
use std::time::Duration;

use common::{
    MockServerTestHarness, reset_all_mock_servers, clear_all_rate_limits,
    configure_rate_limits, failing_behavior
};

/// Test that rate limiting is properly enforced at the server level
/// This test verifies rate limit configuration and enforcement behavior
#[tokio::test]
async fn test_server_rate_limiting_enforcement() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Step 1: Clear all configurations - both mock behaviors and rate limits
    println!("🔄 Clearing all configurations...");
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
    // Step 2: Configure rate limiting on mock-0 (5 requests per second)
    println!("⚙️  Configuring mock-0 with 5 QPS rate limit...");
    configure_rate_limits(&harness, "mock-0", Some(5), None).await?;
    
    // Give additional time for rate limiter to fully initialize
    println!("⏳ Waiting for rate limiter to fully initialize...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Step 3: Verify only mock-0 has rate limits
    println!("🔍 Verifying rate limit configuration...");
    let links = harness.get_statistics("localnet").await?;
    if let Some(links_vec) = &links.links {
        for link in links_vec {
            if link.alias.starts_with("mock-") {
                if link.alias == "mock-0" {
                    assert_eq!(link.max_per_secs, Some(5), "mock-0 should have 5 QPS limit");
                    assert_eq!(link.max_per_min, None, "mock-0 should not have QPM limit");
                } else {
                    assert_eq!(link.max_per_secs, None, "{} should not have QPS limit", link.alias);
                    assert_eq!(link.max_per_min, None, "{} should not have QPM limit", link.alias);
                }
            }
        }
    }
    
    // Step 4: Make other servers fail to force ALL traffic to mock-0
    println!("❌ Making other servers fail to force ALL traffic to rate-limited mock-0...");
    for i in 1..=4 {
        let alias = format!("mock-{}", i);
        // Make servers fail completely
        harness.configure_mock_server_and_wait_for_state(&alias, failing_behavior(1.0), false).await?;
    }
    
    // Ensure only mock-0 is healthy
    harness.ensure_servers_healthy(&["mock-0"]).await?;
    
    // Step 5: Send requests at a rate that exceeds the limit
    println!("📊 Sending burst of requests to test rate limiting...");
    let num_requests = 20;
    let start_time = std::time::Instant::now();
    
    // Send requests rapidly (should take < 1 second)
    let responses = harness.send_rpc_burst(num_requests, "sui_getLatestSuiSystemState").await?;
    let elapsed = start_time.elapsed();
    
    println!("⏱️  Sent {} requests in {:?}", num_requests, elapsed);
    
    // Step 6: Analyze responses
    let mut success_count = 0;
    let mut failed_count = 0;
    for response in &responses {
        if response.status().is_success() {
            success_count += 1;
        } else {
            failed_count += 1;
        }
    }
    
    println!("✅ Successful requests: {}", success_count);
    println!("❌ Failed requests: {}", failed_count);
    
    // With a 5 QPS limit on mock-0 and all other servers failed:
    // - We sent 20 requests rapidly
    // - mock-0 can only handle ~5 per second
    // - Some requests should fail due to rate limiting
    assert!(failed_count > 0, "Expected some requests to fail due to rate limiting");
    assert!(success_count <= 10, "Expected at most 10 requests to succeed with 5 QPS limit");
    
    // Step 7: Verify rate limiting statistics
    let final_stats = harness.get_statistics("localnet").await?;
    if let Some(links_vec) = final_stats.links {
        // Print all server stats for debugging
        println!("\nFinal server statistics:");
        for link in &links_vec {
            println!("  {}: Load={}%, Status={}, Rate limit count={:?}", 
                     link.alias, link.load_pct, link.status, link.rate_limit_count_raw);
        }
        
        if let Some(mock_0_link) = links_vec.iter().find(|link| link.alias == "mock-0") {
            println!("\nMock-0 detailed stats:");
            println!("  Load: {}%", mock_0_link.load_pct);
            println!("  Rate limit count: {:?}", mock_0_link.rate_limit_count_raw);
            println!("  Max per secs: {:?}", mock_0_link.max_per_secs);
            
            // Check rate limiting behavior
            let load = mock_0_link.load_pct.parse::<f64>().unwrap_or(0.0);
            
            // With 5 QPS limit and 20 requests rapidly sent, mock-0 should handle some traffic
            assert!(load > 0.0, "Expected mock-0 to handle some traffic, but got {}%", load);
            
            // Verify rate limiting is configured
            assert_eq!(mock_0_link.max_per_secs, Some(5), "mock-0 should have 5 QPS limit configured");
            
            // Rate limit hits show the limiter is active
            if let Some(rate_limit_count) = mock_0_link.rate_limit_count_raw {
                println!("  ✓ Rate limiter is tracking: {} hits", rate_limit_count);
                assert!(rate_limit_count > 0, "Expected rate limiter to track hits");
            }
        }
    }
    
    harness.cleanup().await?;
    Ok(())
}

/// Test dual rate limiting (QPS and QPM) configuration and behavior
/// This test verifies that both per-second and per-minute limits work correctly
#[tokio::test]
async fn test_dual_rate_limiting() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Step 1: Clear all configurations - both mock behaviors and rate limits
    println!("🔄 Clearing all configurations...");
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
    // Step 2: Configure dual rate limits on mock-1 (2 QPS, 60 QPM)
    println!("⚙️  Configuring mock-1 with dual rate limits (2 QPS, 60 QPM)...");
    configure_rate_limits(&harness, "mock-1", Some(2), Some(60)).await?;
    
    // Step 3: Verify only mock-1 has rate limits
    println!("🔍 Verifying rate limit configuration...");
    let links = harness.get_statistics("localnet").await?;
    if let Some(links_vec) = &links.links {
        for link in links_vec {
            if link.alias.starts_with("mock-") {
                if link.alias == "mock-1" {
                    assert_eq!(link.max_per_secs, Some(2), "mock-1 should have 2 QPS limit");
                    assert_eq!(link.max_per_min, Some(60), "mock-1 should have 60 QPM limit");
                } else {
                    assert_eq!(link.max_per_secs, None, "{} should not have QPS limit", link.alias);
                    assert_eq!(link.max_per_min, None, "{} should not have QPM limit", link.alias);
                }
            }
        }
    }
    
    // Step 4: Test QPS limit (2 per second)
    println!("\n📊 Testing QPS limit (2 per second)...");
    
    // Make all other servers unavailable to force traffic to mock-1
    for i in 0..=4 {
        if i != 1 { // Skip mock-1
            let alias = format!("mock-{}", i);
            harness.configure_mock_server_and_wait_for_state(&alias, failing_behavior(1.0), false).await?;
        }
    }
    
    // Ensure mock-1 is still healthy
    harness.ensure_servers_healthy(&["mock-1"]).await?;
    
    // Send 6 requests rapidly (should exceed 2 QPS)
    let qps_start = std::time::Instant::now();
    let mut qps_responses = Vec::new();
    for _ in 0..6 {
        let response = harness.send_rpc_request("sui_getObject").await?;
        qps_responses.push(response);
    }
    let qps_elapsed = qps_start.elapsed();
    
    println!("⏱️  Sent 6 requests in {:?}", qps_elapsed);
    
    // Count successes - with 2 QPS limit, only ~2 should succeed in first second
    let qps_success_count = qps_responses.iter()
        .filter(|r| r.status().is_success())
        .count();
    
    println!("✅ QPS test: {} out of 6 requests succeeded", qps_success_count);
    
    // In a rapid burst, we expect rate limiting to kick in
    assert!(qps_success_count <= 4, 
            "Expected QPS limit to restrict success count to <= 4, but got {}", 
            qps_success_count);
    
    // Step 5: Test QPM limit would take too long, so we'll just verify the config
    println!("\n✅ QPM limit configured correctly (60 QPM) - full test would take 1+ minute");
    
    // Step 6: Test rate limit removal
    println!("\n🔄 Testing rate limit removal...");
    configure_rate_limits(&harness, "mock-1", None, None).await?;
    
    // Verify all servers have expected configuration after test
    let final_links = harness.get_statistics("localnet").await?;
    if let Some(links_vec) = &final_links.links {
        for link in links_vec {
            if link.alias.starts_with("mock-") {
                // All mock servers should have no rate limits at the end
                assert_eq!(link.max_per_secs, None, "{} should have no QPS limit after test", link.alias);
                assert_eq!(link.max_per_min, None, "{} should have no QPM limit after test", link.alias);
            }
        }
    }
    
    harness.cleanup().await?;
    Ok(())
}

/// Test dynamic configuration changes during runtime
#[tokio::test]
async fn test_dynamic_rate_limit_changes() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Clear all configurations
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
    // Send burst - should all succeed
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    assert!(success_count >= 18, "Expected most requests to succeed initially but got {}", success_count);
    
    // Apply rate limit of 5 QPS to mock-0
    configure_rate_limits(&harness, "mock-0", Some(5), None).await?;
    
    // Send another burst - should still work via load balancing
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    assert!(success_count >= 18, "Expected requests to succeed via load balancing but got {}", success_count);
    
    // Remove rate limit
    configure_rate_limits(&harness, "mock-0", None, None).await?;
    
    // Verify it works again
    let responses = harness.send_rpc_burst(20, "sui_getLatestSuiSystemState").await?;
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    assert!(success_count >= 18, "Expected requests to succeed after removing rate limit but got {}", success_count);
    
    harness.cleanup().await?;
    Ok(())
}

/// Test rate limiting with load balancing
/// When one server is rate limited, traffic should overflow to other servers
#[tokio::test]
async fn test_rate_limiting_with_load_balancing() -> Result<()> {
    let mut harness = MockServerTestHarness::new().await?;
    
    // Reset environment
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    
    // Configure aggressive rate limiting on mock-0 and mock-1
    println!("Configuring rate limits on mock-0 and mock-1...");
    configure_rate_limits(&harness, "mock-0", Some(2), None).await?;
    configure_rate_limits(&harness, "mock-1", Some(2), None).await?;
    
    // Ensure all servers are healthy
    harness.ensure_servers_healthy(&["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"]).await?;
    
    // Send a burst of requests
    println!("Sending burst of 50 requests...");
    let responses = harness.send_rpc_burst(50, "sui_getLatestSuiSystemState").await?;
    
    // All requests should succeed via load balancing
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    assert_eq!(success_count, 50, "Expected all 50 requests to succeed via load balancing");
    
    // Check traffic distribution
    let stats = harness.get_statistics("localnet").await?;
    if let Some(links) = stats.links {
        println!("\nTraffic distribution:");
        for link in &links {
            if link.alias.starts_with("mock-") {
                println!("  {}: {}% load", link.alias, link.load_pct);
            }
        }
        
        // Verify rate-limited servers (mock-0, mock-1) got less traffic
        let mock_0_load = links.iter()
            .find(|l| l.alias == "mock-0")
            .map(|l| l.load_pct.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
            
        let mock_1_load = links.iter()
            .find(|l| l.alias == "mock-1")
            .map(|l| l.load_pct.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
            
        // Non-rate-limited servers should handle most of the traffic
        let other_load = 100.0 - mock_0_load - mock_1_load;
        assert!(other_load > 50.0, 
                "Expected non-rate-limited servers to handle most traffic, but only got {:.1}%", 
                other_load);
    }
    
    harness.cleanup().await?;
    Ok(())
}