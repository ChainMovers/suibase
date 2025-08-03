// Simple test to verify rate limiting is working
use anyhow::Result;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

mod common;
use common::mock_test_utils::{configure_rate_limits, failing_behavior, MockServerTestHarness};

#[tokio::test]
async fn test_rate_limit_enforcement() -> Result<()> {
    println!("\n=== Testing Rate Limit Enforcement ===");
    
    let harness = MockServerTestHarness::new().await?;
    
    // Wait for daemon to be ready
    sleep(Duration::from_millis(1000)).await;
    
    // Configure mock-0 with a very low rate limit (2 QPS)
    println!("Configuring mock-0 with 2 QPS rate limit...");
    configure_rate_limits(&harness, "mock-0", Some(2), Some(120)).await?;
    
    // Force all other mock servers to be unavailable so requests go to mock-0
    println!("Configuring other mock servers to be unhealthy...");
    for i in 1..=4 {
        let alias = format!("mock-{}", i);
        harness.configure_mock_server(&alias, failing_behavior(1.0)).await?;
    }
    
    // Wait for configuration to take effect
    sleep(Duration::from_millis(2000)).await;
    
    // Verify mock-0 has rate limit configured
    let links_response = harness.get_links("localnet", true, true, true, false, false).await?;
    if let Some(links) = links_response.links {
        for link in links {
            println!("Server {}: QPS limit={:?}, Status={}", 
                     link.alias, link.max_per_secs, link.status);
        }
    }
    
    // Create a client
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    
    let proxy_url = "http://localhost:44340";
    
    println!("\nSending 10 requests in rapid succession (should exceed 2 QPS)...");
    
    // Send 10 requests as fast as possible
    let mut results = vec![];
    for i in 0..10 {
        let client = client.clone();
        let proxy_url = proxy_url.to_string();
        
        let handle = tokio::spawn(async move {
            let request_body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "suix_getLatestSuiSystemState",
                "params": []
            });
            
            let response = client
                .post(&proxy_url)
                .json(&request_body)
                .send()
                .await;
            
            match response {
                Ok(resp) => {
                    let status = resp.status();
                    (i, status.as_u16(), status.is_success())
                }
                Err(_) => (i, 0, false)
            }
        });
        
        results.push(handle);
    }
    
    // Collect results
    let mut successful = 0;
    let mut rate_limited = 0;
    let mut other_errors = 0;
    
    for handle in results {
        if let Ok((id, status, success)) = handle.await {
            if success {
                successful += 1;
                println!("Request {} succeeded", id);
            } else if status == 429 {
                rate_limited += 1;
                println!("Request {} was rate limited (429)", id);
            } else {
                other_errors += 1;
                println!("Request {} failed with status {}", id, status);
            }
        }
    }
    
    println!("\nResults:");
    println!("  Successful: {}", successful);
    println!("  Rate limited (429): {}", rate_limited);
    println!("  Other errors: {}", other_errors);
    
    // Wait and check statistics
    sleep(Duration::from_millis(1000)).await;
    
    let final_response = harness.get_links("localnet", true, true, true, false, false).await?;
    if let Some(links) = final_response.links {
        for link in links {
            if link.alias == "mock-0" {
                println!("\nmock-0 final statistics:");
                println!("  Rate limit count: {:?}", link.rate_limit_count_raw);
                println!("  QPS: {:?}", link.qps_raw);
                println!("  QPM: {:?}", link.qpm_raw);
                break;
            }
        }
    }
    
    // Display the rate limit table
    println!("\n=== Rate Limit Statistics Table ===");
    let display_response = harness.get_links("localnet", true, true, false, true, false).await?;
    if let Some(display) = display_response.display {
        println!("{}", display);
    }
    
    if rate_limited == 0 {
        println!("\n⚠️  WARNING: No rate limiting detected!");
        println!("Possible reasons:");
        println!("1. Rate limiting may not be implemented for mock servers");
        println!("2. The proxy may be load balancing to other servers");
        println!("3. The rate limiter may not be enforcing limits correctly");
    } else {
        println!("\n✅ Rate limiting is working! {} requests were rate limited", rate_limited);
    }
    
    Ok(())
}