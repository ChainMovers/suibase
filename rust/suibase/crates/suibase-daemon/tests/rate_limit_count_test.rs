// Integration tests for rate limit count increment verification

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

mod common;
use common::mock_test_utils::{configure_rate_limits, MockServerTestHarness};
use suibase_daemon::shared_types::MockServerBehavior;

#[tokio::test]
async fn test_rate_limit_count_increments() -> Result<()> {
    println!("\n=== Testing Rate Limit Count Increments ===");

    let harness = MockServerTestHarness::new().await?;

    // Step 1: Disable all mock servers except mock-0 to force all traffic through it
    println!("Disabling mock-1, mock-2, mock-3, and mock-4 to isolate all load on mock-0...");
    harness
        .modify_config_and_wait(|config| {
            if let Some(links) = config.get_mut("links").and_then(|l| l.as_sequence_mut()) {
                for link in links {
                    if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                        if ["mock-1", "mock-2", "mock-3", "mock-4"].contains(&alias) {
                            if let Some(mapping) = link.as_mapping_mut() {
                                mapping.insert("selectable".into(), serde_yaml::Value::Bool(false));
                            }
                        }
                    }
                }
            }
        })
        .await?;

    // Step 2: Configure mock-0 with a low rate limit (10 QPS)
    println!("\nConfiguring mock-0 with 10 QPS rate limit...");
    configure_rate_limits(&harness, "mock-0", Some(10), Some(600)).await?;

    // Define a reusable async function for the test logic
    async fn run_burst_and_verify(harness: &MockServerTestHarness) -> Result<()> {
        // Get initial statistics for mock-0
        let initial_stats = harness.get_statistics("localnet").await?;
        let mock_0_initial = initial_stats
            .links
            .as_ref()
            .and_then(|links| links.iter().find(|l| l.alias == "mock-0"))
            .ok_or_else(|| anyhow::anyhow!("mock-0 not found in initial stats"))?;
        let initial_rate_limit_count = mock_0_initial.rate_limit_count_raw.unwrap_or(0);
        println!(
            "Initial rate limit count for mock-0: {}",
            initial_rate_limit_count
        );

        // Send a burst of 50 requests to trigger rate limiting
        println!("\nSending 50 rapid requests to trigger rate limiting...");
        let responses = harness.send_rpc_burst(50, "suix_getLatestSuiSystemState").await?;

        let successful_requests = responses.iter().filter(|r| r.status().is_success()).count();
        let failed_requests = responses.iter().filter(|r| !r.status().is_success()).count();

        println!("Successful requests: {}", successful_requests);
        println!("Failed requests: {}", failed_requests);
        
        // Debug: Check server availability
        let stats_during = harness.get_statistics("localnet").await?;
        if let Some(links) = stats_during.links.as_ref() {
            println!("\nServer availability during burst:");
            for link in links {
                if link.alias.starts_with("mock-") {
                    println!("  {} - status: {}, selectable: {:?}, rate limits: QPS={:?}, QPM={:?}", 
                             link.alias, link.status, link.selectable, 
                             link.max_per_secs, link.max_per_min);
                }
            }
        }
        
        // We expect some requests to succeed (at least from mock-1)
        // The proxy doesn't return 429 - it handles rate limiting internally
        assert!(
            successful_requests > 0,
            "Expected at least some successful requests"
        );

        // Wait longer for statistics to update
        println!("\nWaiting for statistics to update...");
        sleep(Duration::from_secs(2)).await;

        // Get updated statistics and verify the count has increased
        let updated_stats = harness.get_statistics("localnet").await?;
        let mock_0_updated = updated_stats
            .links
            .as_ref()
            .and_then(|links| links.iter().find(|l| l.alias == "mock-0"))
            .ok_or_else(|| anyhow::anyhow!("mock-0 not found in updated stats"))?;
        let updated_rate_limit_count = mock_0_updated.rate_limit_count_raw.unwrap_or(0);
        println!(
            "Updated rate limit count for mock-0: {}",
            updated_rate_limit_count
        );

        let count_increase = updated_rate_limit_count.saturating_sub(initial_rate_limit_count);
        println!("Rate limit count increased by: {}", count_increase);

        assert!(
            count_increase > 0,
            "Rate limit count should have increased. Initial: {}, Updated: {}",
            initial_rate_limit_count,
            updated_rate_limit_count
        );

        // The increase should be significant since we sent many requests exceeding the rate limit
        // We can't predict exact count as it depends on timing and server selection
        println!(
            "Rate limit events detected: {} (from burst of 50 requests with 10 QPS limit)",
            count_increase
        );
        Ok(())
    }

    // --- First Run ---
    println!("\n--- Running First Burst ---");
    run_burst_and_verify(&harness).await?;

    // --- Verify QPS Recovery ---
    println!("\n--- Verifying QPS Recovery ---");
    println!("Waiting 5 seconds for QPS to drop...");
    sleep(Duration::from_secs(1)).await;

    let recovery_stats = harness.get_statistics("localnet").await?;
    let mock_0_recovery = recovery_stats
        .links
        .as_ref()
        .and_then(|links| links.iter().find(|l| l.alias == "mock-0"))
        .ok_or_else(|| anyhow::anyhow!("mock-0 not found in recovery stats"))?;

    // Check the QPS has been tracked
    let qps_raw = mock_0_recovery.qps_raw.unwrap_or(0);
    println!("QPS for mock-0 after recovery: {}", qps_raw);
    
    // After 1 second, the QPS should have dropped significantly from the burst
    // The exact value depends on timing, but it should be less than the configured limit
    assert!(
        qps_raw <= 10,
        "Expected QPS to be at or below the configured limit of 10, but it was {}",
        qps_raw
    );

    // --- Second Run ---
    println!("\n--- Running Second Burst ---");
    run_burst_and_verify(&harness).await?;

    // --- Cleanup ---
    println!("\n--- Resetting to Baseline Configuration ---");
    harness.reset_to_baseline_config().await?;

    println!("\n✅ Rate limit count increment test PASSED");
    Ok(())
}

#[tokio::test]
async fn test_rate_limit_with_fallback_server() -> Result<()> {
    println!("\n=== Testing Rate Limit Count with Fallback Server ===");
    
    let harness = MockServerTestHarness::new().await?;
    
    // Step 1: Disable mock-2, mock-3, and mock-4 but keep mock-1 available
    println!("Disabling mock-2, mock-3, and mock-4 (keeping mock-1 as fallback)...");
    harness
        .modify_config_and_wait(|config| {
            if let Some(links) = config.get_mut("links").and_then(|l| l.as_sequence_mut()) {
                for link in links {
                    if let Some(alias) = link.get("alias").and_then(|a| a.as_str()) {
                        if ["mock-2", "mock-3", "mock-4"].contains(&alias) {
                            if let Some(mapping) = link.as_mapping_mut() {
                                mapping.insert("selectable".into(), serde_yaml::Value::Bool(false));
                            }
                        }
                    }
                }
            }
        })
        .await?;
    
    // Step 2: Configure mock servers to be operational
    println!("\nConfiguring mock servers...");
    // Configure mock-0 and mock-1 with default behavior (healthy)
    harness.configure_mock_server("mock-0", MockServerBehavior::default()).await?;
    harness.configure_mock_server("mock-1", MockServerBehavior::default()).await?;
    
    // Step 3: Configure mock-0 with a low rate limit (5 QPS to make it easier to exceed)
    println!("\nConfiguring mock-0 with 5 QPS rate limit...");
    configure_rate_limits(&harness, "mock-0", Some(5), Some(300)).await?;
    
    // Wait for configuration to settle
    sleep(Duration::from_millis(1000)).await;
    
    // Check server availability before the test
    let pre_test_stats = harness.get_statistics("localnet").await?;
    if let Some(links) = pre_test_stats.links.as_ref() {
        println!("\nServer availability before test:");
        for link in links {
            if link.alias.starts_with("mock-") {
                println!("  {} - status: {}, selectable: {:?}", 
                         link.alias, link.status, link.selectable);
            }
        }
    }
    
    // Ensure mock-0 and mock-1 are healthy before proceeding
    println!("\nEnsuring mock-0 and mock-1 are healthy...");
    harness.ensure_servers_healthy(&["mock-0", "mock-1"]).await?;
    
    // Get initial statistics
    let initial_stats = harness.get_statistics("localnet").await?;
    let mock_0_initial = initial_stats
        .links
        .as_ref()
        .and_then(|links| links.iter().find(|l| l.alias == "mock-0"))
        .ok_or_else(|| anyhow::anyhow!("mock-0 not found in initial stats"))?;
    let initial_rate_limit_count = mock_0_initial.rate_limit_count_raw.unwrap_or(0);
    
    println!("Initial rate limit count for mock-0: {}", initial_rate_limit_count);
    
    // Send 2 distinct bursts to verify rate limiting behavior
    // First burst should trigger rate limiting on mock-0
    println!("\n=== First Burst ===");
    println!("Sending 50 rapid requests...");
    let responses_burst1 = harness.send_rpc_burst(50, "suix_getLatestSuiSystemState").await?;
    
    let successful_1 = responses_burst1.iter().filter(|r| r.status().is_success()).count();
    println!("First burst: {} successful requests", successful_1);
    
    // Wait 2 seconds for rate limiter to fully reset (> 1 second for QPS reset)
    println!("\nWaiting 2 seconds for rate limiter to reset...");
    sleep(Duration::from_secs(2)).await;
    
    // Second burst should also trigger rate limiting
    println!("\n=== Second Burst ===");
    println!("Sending another 50 rapid requests...");
    let responses_burst2 = harness.send_rpc_burst(50, "suix_getLatestSuiSystemState").await?;
    
    let successful_2 = responses_burst2.iter().filter(|r| r.status().is_success()).count();
    println!("Second burst: {} successful requests", successful_2);
    
    // Combine all responses
    let mut all_responses = responses_burst1;
    all_responses.extend(responses_burst2);
    let responses = all_responses;
    
    let successful_requests = responses.iter().filter(|r| r.status().is_success()).count();
    let failed_requests = responses.iter().filter(|r| !r.status().is_success()).count();
    
    println!("Successful requests: {}", successful_requests);
    println!("Failed requests: {}", failed_requests);
    
    // With mock-1 available, all requests should succeed
    assert_eq!(
        failed_requests, 0,
        "Expected no failed requests when fallback server is available"
    );
    assert_eq!(
        successful_requests, 100,
        "Expected all 100 requests to succeed with fallback server"
    );
    
    // Wait for statistics to update
    println!("\nWaiting for statistics to update...");
    sleep(Duration::from_secs(2)).await;
    
    // Get updated statistics
    let updated_stats = harness.get_statistics("localnet").await?;
    
    // Check mock-0 stats
    let mock_0_updated = updated_stats
        .links
        .as_ref()
        .and_then(|links| links.iter().find(|l| l.alias == "mock-0"))
        .ok_or_else(|| anyhow::anyhow!("mock-0 not found in updated stats"))?;
    
    let updated_rate_limit_count = mock_0_updated.rate_limit_count_raw.unwrap_or(0);
    
    println!("\nUpdated rate limit count for mock-0: {}", updated_rate_limit_count);
    
    let rate_limit_increase = updated_rate_limit_count.saturating_sub(initial_rate_limit_count);
    
    println!("Rate limit count increased by: {}", rate_limit_increase);
    
    // Verify rate limiting occurred - expecting at least 2 increments (one per burst)
    assert!(
        rate_limit_increase >= 2,
        "Rate limit count should have increased at least twice (once per burst). Initial: {}, Updated: {}, Increase: {}",
        initial_rate_limit_count,
        updated_rate_limit_count,
        rate_limit_increase
    );
    
    println!("✅ Rate limiting detected as expected with {} increments", rate_limit_increase);
    
    // Check both servers' statistics
    println!("\n=== Server Statistics After Burst ===");
    if let Some(links) = updated_stats.links.as_ref() {
        for link in links {
            if link.alias == "mock-0" || link.alias == "mock-1" {
                println!("{} - QPS: {:?}, QPM: {:?}, rate_limit_count: {:?}", 
                         link.alias, 
                         link.qps_raw,
                         link.qpm_raw,
                         link.rate_limit_count_raw);
            }
        }
    }
    
    // Display the links with rate limit info
    println!("\n=== Server Statistics After Test ===");
    let display_response = harness.get_links("localnet", true, true, false, true, false).await?;
    if let Some(display) = display_response.display {
        println!("{}", display);
    }
    
    // Reset configuration
    println!("\n--- Resetting to Baseline Configuration ---");
    harness.reset_to_baseline_config().await?;
    
    println!("\n✅ Rate limit with fallback server test PASSED");
    Ok(())
}
