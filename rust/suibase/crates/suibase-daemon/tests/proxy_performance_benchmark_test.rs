// Performance benchmarking tests for the proxy server
// Only runs when the "benchmarks" feature is enabled

use anyhow::Result;
use reqwest::Client;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use std::io::Write;

mod common;
use common::mock_test_utils::MockServerTestHarness;

#[cfg_attr(not(feature = "benchmarks"), ignore)]
#[tokio::test]
async fn test_proxy_server_performance_benchmark() -> Result<()> {
    println!("\n=== Proxy Server Performance Benchmark ===");

    let _harness = MockServerTestHarness::new().await?;

    // Wait for daemon to be ready
    sleep(Duration::from_millis(1000)).await;

    // Create a client with connection pooling for better performance
    let client = Client::builder()
        .pool_max_idle_per_host(100)
        .timeout(Duration::from_secs(10))
        .build()?;

    let proxy_url = "http://localhost:44340"; // localnet proxy port

    // Warm up the connection pool
    println!("\nWarming up connection pool...");
    for _ in 0..10 {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "suix_getLatestSuiSystemState",
            "params": []
        });

        let _ = client.post(proxy_url).json(&request_body).send().await;
    }

    // Run benchmark with different concurrency levels
    let concurrency_levels = vec![1, 10, 50, 100, 200, 500];
    let requests_per_test = 1000;
    let iterations_per_level = 10; // Run 10 iterations for each concurrency level

    // Store averaged results for summary
    let mut averaged_qps_results = Vec::new();
    let mut averaged_latency_results = Vec::new();

    for concurrency in &concurrency_levels {
        let concurrency = *concurrency;
        println!("\n--- Benchmarking with {} concurrent connections ({} iterations) ---", 
                 concurrency, iterations_per_level);

        let mut iteration_qps = Vec::new();
        let mut iteration_latencies = Vec::new();

        for iteration in 1..=iterations_per_level {
            if iteration > 1 {
                // Brief pause between iterations
                sleep(Duration::from_millis(100)).await;
            }
            
            print!("  Iteration {}/{}... ", iteration, iterations_per_level);
            std::io::stdout().flush().unwrap();

            let start_time = Instant::now();
            let successful_requests = Arc::new(AtomicU64::new(0));
            let failed_requests = Arc::new(AtomicU64::new(0));
            let total_latency_ms = Arc::new(AtomicU64::new(0));

            let mut handles = vec![];
            let requests_per_task = requests_per_test / concurrency;

            for task_id in 0..concurrency {
                let client = client.clone();
                let proxy_url = proxy_url.to_string();
                let successful = Arc::clone(&successful_requests);
                let failed = Arc::clone(&failed_requests);
                let latency = Arc::clone(&total_latency_ms);

                let handle = tokio::spawn(async move {
                    for i in 0..requests_per_task {
                        let request_start = Instant::now();

                        let request_body = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": task_id * requests_per_task + i,
                            "method": "suix_getLatestSuiSystemState",
                            "params": []
                        });

                        match client.post(&proxy_url).json(&request_body).send().await {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    successful.fetch_add(1, Ordering::Relaxed);
                                    let request_latency = request_start.elapsed().as_millis() as u64;
                                    latency.fetch_add(request_latency, Ordering::Relaxed);
                                } else {
                                    failed.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            Err(_) => {
                                failed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                });

                handles.push(handle);
            }

            // Wait for all tasks to complete
            for handle in handles {
                handle.await?;
            }

            let total_duration = start_time.elapsed();
            let total_successful = successful_requests.load(Ordering::Relaxed);
            let total_failed = failed_requests.load(Ordering::Relaxed);
            let avg_latency_ms = if total_successful > 0 {
                total_latency_ms.load(Ordering::Relaxed) as f64 / total_successful as f64
            } else {
                0.0
            };

            let qps = total_successful as f64 / total_duration.as_secs_f64();
            
            iteration_qps.push(qps);
            iteration_latencies.push(avg_latency_ms);
            
            println!("QPS: {:.0}, Latency: {:.2}ms", qps, avg_latency_ms);
        }

        // Calculate averages for this concurrency level
        let avg_qps = iteration_qps.iter().sum::<f64>() / iteration_qps.len() as f64;
        let avg_latency = iteration_latencies.iter().sum::<f64>() / iteration_latencies.len() as f64;
        
        averaged_qps_results.push((concurrency, avg_qps));
        averaged_latency_results.push((concurrency, avg_latency));

        println!("\nAverage for {} connections: QPS: {:.0}, Latency: {:.2}ms", 
                 concurrency, avg_qps, avg_latency);
    }

    println!("\n=== Performance Benchmark Summary ===");

    // Calculate overall statistics from averaged results
    let mut min_qps = f64::MAX;
    let mut max_qps: f64 = 0.0;
    let mut total_qps = 0.0;
    let mut min_latency = f64::MAX;
    let mut max_latency: f64 = 0.0;
    let mut total_latency = 0.0;

    for (_, qps) in &averaged_qps_results {
        min_qps = min_qps.min(*qps);
        max_qps = max_qps.max(*qps);
        total_qps += qps;
    }

    for (_, latency) in &averaged_latency_results {
        min_latency = min_latency.min(*latency);
        max_latency = max_latency.max(*latency);
        total_latency += latency;
    }

    let avg_qps = total_qps / averaged_qps_results.len() as f64;
    let avg_latency = total_latency / averaged_latency_results.len() as f64;

    // Combined Performance Summary
    println!("\n📊 Performance Summary by Concurrency Level (averaged over {} iterations):", iterations_per_level);
    println!("┌────────────┬──────────────┬──────────────┬──────────────┐");
    println!("│ Concurrent │   QPS Total  │   QPS/Conn   │ Avg Latency  │");
    println!("│ Connections│              │              │              │");
    println!("├────────────┼──────────────┼──────────────┼──────────────┤");
    for i in 0..averaged_qps_results.len() {
        let (concurrency, qps) = averaged_qps_results[i];
        let qps_per_conn = qps / (concurrency as f64);
        let (_, latency) = averaged_latency_results[i];
        println!("│ {:>10} │ {:>12.0} │ {:>12.2} │{:>10.2} ms │",
                 concurrency,
                 qps,
                 qps_per_conn,
                 latency);
    }
    println!("└────────────┴──────────────┴──────────────┴──────────────┘");

    // Find best QPS and latency
    if let Some((best_qps_concurrency, best_qps)) = averaged_qps_results.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
        println!("\n🏆 Best QPS: {:.0} at {} concurrent connections", best_qps, best_qps_concurrency);
    }
    if let Some((best_latency_concurrency, best_latency)) = averaged_latency_results.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
        println!("🏆 Best Latency: {:.2}ms at {} concurrent connection{}",
                 best_latency,
                 best_latency_concurrency,
                 if *best_latency_concurrency == 1 { "" } else { "s" });
    }

    // Overall Performance Summary with Worst/Average/Best as rows
    println!("\n📊 Overall Performance Summary:");
    println!("┌─────────────┬───────────────┬───────────────┐");
    println!("│             │      QPS      │  Latency (ms) │");
    println!("├─────────────┼───────────────┼───────────────┤");
    println!("│ Worst       │{:>12.0}   │{:>12.2}   │",
             min_qps, max_latency);
    println!("│ Average     │{:>12.0}   │{:>12.2}   │",
             avg_qps, avg_latency);
    println!("│ Best        │{:>12.0}   │{:>12.2}   │",
             max_qps, min_latency);
    println!("└─────────────┴───────────────┴───────────────┘");

    println!("\n✅ Proxy server performance benchmark PASSED");

    Ok(())
}

#[cfg_attr(not(feature = "benchmarks"), ignore)]
#[tokio::test]
async fn test_mock_server_unlimited_performance() -> Result<()> {
    println!("\n=== Mock Server Maximum Performance Test ===");

    let harness = MockServerTestHarness::new().await?;

    // Wait for daemon to be ready
    sleep(Duration::from_millis(1000)).await;

    // For this test, we'll use mock-2 which should have unlimited rate
    // Let's verify mock-2 exists and has unlimited rate
    let links_response = harness.get_links("localnet", true, true, true, false, false).await?;

    let mut mock_2_found = false;
    if let Some(links) = links_response.links {
        for link in links {
            if link.alias == "mock-2" {
                mock_2_found = true;
                println!("Found mock-2 with rate limits: QPS={:?}, QPM={:?}",
                         link.max_per_secs, link.max_per_min);
                break;
            }
        }
    }

    if !mock_2_found {
        println!("mock-2 not found, using default proxy for performance test");
    }

    // Create high-performance client
    let client = Client::builder()
        .pool_max_idle_per_host(200)
        .timeout(Duration::from_secs(30))
        .build()?;

    let proxy_url = "http://localhost:44340";

    // Warm up
    println!("\nWarming up...");
    for _ in 0..50 {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "suix_getLatestSuiSystemState",
            "params": []
        });
        let _ = client.post(proxy_url).json(&request_body).send().await;
    }

    // Maximum performance test
    let concurrency = 200;
    let total_requests = 10000;
    let requests_per_task = total_requests / concurrency;

    println!("\nRunning maximum performance test:");
    println!("  Concurrency: {}", concurrency);
    println!("  Total requests: {}", total_requests);

    let start_time = Instant::now();
    let successful_requests = Arc::new(AtomicU64::new(0));
    let failed_requests = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for task_id in 0..concurrency {
        let client = client.clone();
        let proxy_url = proxy_url.to_string();
        let successful = Arc::clone(&successful_requests);
        let failed = Arc::clone(&failed_requests);

        let handle = tokio::spawn(async move {
            for i in 0..requests_per_task {
                let request_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": task_id * requests_per_task + i,
                    "method": "suix_getLatestSuiSystemState",
                    "params": []
                });

                match client.post(&proxy_url).json(&request_body).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            successful.fetch_add(1, Ordering::Relaxed);
                        } else {
                            failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await?;
    }

    let duration = start_time.elapsed();
    let total_successful = successful_requests.load(Ordering::Relaxed);
    let total_failed = failed_requests.load(Ordering::Relaxed);

    let max_qps = total_successful as f64 / duration.as_secs_f64();

    println!("\n=== Maximum Performance Results ===");
    println!("Duration: {:?}", duration);
    println!("Successful requests: {}", total_successful);
    println!("Failed requests: {}", total_failed);
    println!("Maximum QPS achieved: {:.0}", max_qps);
    println!("Success rate: {:.1}%",
             (total_successful as f64 / total_requests as f64) * 100.0);

    // Performance assertion
    assert!(
        max_qps > 100.0,
        "Expected at least 100 QPS for unlimited mock server, got {:.0}",
        max_qps
    );

    // Calculate average latency for max performance test
    let avg_latency_ms = duration.as_millis() as f64 / total_successful as f64;

    println!("\n📊 Maximum Performance Summary:");
    println!("┌───────────────────────┬─────────────────────┐");
    println!("│ Metric                │ Value               │");
    println!("├───────────────────────┼─────────────────────┤");
    println!("│ Maximum QPS           │{:>17} {:<3}│", max_qps as u64, "");
    println!("│ Average Latency       │{:>17.2} {:<3}│", avg_latency_ms, "ms");
    println!("└───────────────────────┴─────────────────────┘");

    println!("\n✅ Mock server maximum performance test PASSED");
    println!("The proxy server can handle {} QPS with unlimited mock backends", max_qps as u64);

    Ok(())
}