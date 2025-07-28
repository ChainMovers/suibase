// Benchmark tests for rate limiter performance
//
// This module contains performance tests that can be run manually
// to verify the rate limiter's behavior under high load.

use suibase_daemon::rate_limiter::RateLimiter;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[test]
#[ignore] // Use --ignored to run this test
fn bench_high_concurrency() {
    println!("Starting high concurrency benchmark...");
    
    let limiter = Arc::new(RateLimiter::new(1000)); // 1000 tokens per second
    let num_threads = 100;
    let attempts_per_thread = 100;
    let total_attempts = num_threads * attempts_per_thread;
    
    let successes = Arc::new(AtomicU64::new(0));
    let failures = Arc::new(AtomicU64::new(0));
    
    // Wait for initial tokens
    thread::sleep(Duration::from_millis(1100));
    
    let start_time = Instant::now();
    let mut handles = vec![];
    
    // Spawn threads that hammer the rate limiter
    for thread_id in 0..num_threads {
        let limiter_clone = Arc::clone(&limiter);
        let successes_clone = Arc::clone(&successes);
        let failures_clone = Arc::clone(&failures);
        
        let handle = thread::spawn(move || {
            let mut local_successes = 0;
            let mut local_failures = 0;
            
            for attempt in 0..attempts_per_thread {
                match limiter_clone.try_acquire_token() {
                    Ok(_) => local_successes += 1,
                    Err(_) => local_failures += 1,
                }
                
                // Small delay to simulate realistic usage
                if attempt % 10 == 0 {
                    thread::sleep(Duration::from_micros(100));
                }
            }
            
            successes_clone.fetch_add(local_successes, Ordering::Relaxed);
            failures_clone.fetch_add(local_failures, Ordering::Relaxed);
            
            println!("Thread {} completed: {} successes, {} failures", 
                     thread_id, local_successes, local_failures);
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start_time.elapsed();
    let final_successes = successes.load(Ordering::Relaxed);
    let final_failures = failures.load(Ordering::Relaxed);
    
    println!("\n=== High Concurrency Benchmark Results ===");
    println!("Duration: {:?}", duration);
    println!("Total attempts: {}", total_attempts);
    println!("Successes: {}", final_successes);
    println!("Failures: {}", final_failures);
    println!("Success rate: {:.2}%", (final_successes as f64 / total_attempts as f64) * 100.0);
    println!("Throughput: {:.2} attempts/sec", total_attempts as f64 / duration.as_secs_f64());
    
    // Verify correctness
    assert_eq!(final_successes + final_failures, total_attempts as u64);
    assert!(final_successes > 0, "Should have some successes");
    assert!(final_failures > 0, "Should have some failures due to rate limiting");
}

#[test]
#[ignore] // Use --ignored to run this test
fn bench_rate_precision() {
    println!("Starting rate precision benchmark...");
    
    let rates_to_test = vec![10, 50, 100, 500, 1000];
    
    for rate in rates_to_test {
        println!("\n--- Testing rate: {} tokens/sec ---", rate);
        
        let limiter = Arc::new(RateLimiter::new(rate));
        let successes = Arc::new(AtomicU64::new(0));
        let num_threads = 10;
        let test_duration = Duration::from_secs(5);
        
        // Wait for initial tokens and let rate stabilize
        thread::sleep(Duration::from_millis(2100));
        
        let start_time = Instant::now();
        let mut handles = vec![];
        
        // Spawn threads that continuously try to acquire tokens
        for _ in 0..num_threads {
            let limiter_clone = Arc::clone(&limiter);
            let successes_clone = Arc::clone(&successes);
            let start = start_time;
            
            let handle = thread::spawn(move || {
                let mut local_successes = 0;
                
                while start.elapsed() < test_duration {
                    if limiter_clone.try_acquire_token().is_ok() {
                        local_successes += 1;
                    }
                    thread::sleep(Duration::from_micros(500)); // Small delay
                }
                
                successes_clone.fetch_add(local_successes, Ordering::Relaxed);
            });
            
            handles.push(handle);
        }
        
        // Wait for test duration plus some buffer
        thread::sleep(test_duration + Duration::from_millis(100));
        
        // Wait for all threads to finish
        for handle in handles {
            handle.join().unwrap();
        }
        
        let final_successes = successes.load(Ordering::Relaxed);
        let actual_rate = final_successes as f64 / test_duration.as_secs_f64();
        let rate_accuracy = (actual_rate / rate as f64) * 100.0;
        
        println!("Expected rate: {} tokens/sec", rate);
        println!("Actual rate: {:.2} tokens/sec", actual_rate);
        println!("Accuracy: {:.1}%", rate_accuracy);
        
        // Rate should be within 15% of expected (allowing for timing variations and startup effects)
        assert!(rate_accuracy >= 85.0 && rate_accuracy <= 115.0, 
                "Rate accuracy {:.1}% is outside acceptable range", rate_accuracy);
    }
}

#[test]
#[ignore] // Use --ignored to run this test  
fn bench_memory_usage() {
    println!("Starting memory usage benchmark...");
    
    // Create many rate limiters to test memory efficiency
    let num_limiters = 10000;
    let mut limiters = Vec::with_capacity(num_limiters);
    
    let start_time = Instant::now();
    
    for i in 0..num_limiters {
        limiters.push(RateLimiter::new((i % 1000 + 1) as u32));
    }
    
    let creation_time = start_time.elapsed();
    
    println!("Created {} rate limiters in {:?}", num_limiters, creation_time);
    println!("Memory per limiter: ~{} bytes", 
            std::mem::size_of::<RateLimiter>());
    
    // Test that they all work
    let mut working_count = 0;
    for limiter in &limiters {
        if limiter.tokens_available() == 0 {
            working_count += 1;
        }
    }
    
    println!("All {} limiters are functional", working_count);
    assert_eq!(working_count, num_limiters);
}

#[test]
#[ignore] // Use --ignored to run this test
fn bench_mixed_workload() {
    println!("Starting mixed workload benchmark...");
    
    // Simulate different types of rate limiters (like different servers)
    let server_configs = vec![
        ("google.com", 100),
        ("cnn.com", 50),
        ("github.com", 200),
        ("stackoverflow.com", 150),
        ("wikipedia.org", 80),
    ];
    
    let mut limiters = Vec::new();
    for (name, rate) in &server_configs {
        limiters.push((name.to_string(), Arc::new(RateLimiter::new(*rate))));
    }
    
    let total_successes = Arc::new(AtomicU64::new(0));
    let total_failures = Arc::new(AtomicU64::new(0));
    
    // Wait for initial tokens
    thread::sleep(Duration::from_millis(1100));
    
    let start_time = Instant::now();
    let mut handles = vec![];
    
    // Spawn threads that randomly select servers to hit
    for thread_id in 0..20 {
        let limiters_clone = limiters.clone();
        let successes_clone = Arc::clone(&total_successes);
        let failures_clone = Arc::clone(&total_failures);
        
        let handle = thread::spawn(move || {
            let mut rng_state: u32 = thread_id as u32; // Simple PRNG state
            let mut local_successes = 0;
            let mut local_failures = 0;
            
            for _ in 0..500 {
                // Simple linear congruential generator for server selection
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let server_idx = (rng_state as usize) % limiters_clone.len();
                
                match limiters_clone[server_idx].1.try_acquire_token() {
                    Ok(_) => local_successes += 1,
                    Err(_) => local_failures += 1,
                }
                
                thread::sleep(Duration::from_micros(200));
            }
            
            successes_clone.fetch_add(local_successes, Ordering::Relaxed);
            failures_clone.fetch_add(local_failures, Ordering::Relaxed);
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start_time.elapsed();
    let final_successes = total_successes.load(Ordering::Relaxed);
    let final_failures = total_failures.load(Ordering::Relaxed);
    let total_attempts = final_successes + final_failures;
    
    println!("\n=== Mixed Workload Results ===");
    println!("Test duration: {:?}", duration);
    println!("Total attempts: {}", total_attempts);
    println!("Total successes: {}", final_successes);
    println!("Total failures: {}", final_failures);
    println!("Overall success rate: {:.2}%", 
            (final_successes as f64 / total_attempts as f64) * 100.0);
    println!("Throughput: {:.2} attempts/sec", 
            total_attempts as f64 / duration.as_secs_f64());
    
    // Check per-server stats
    for (name, limiter) in &limiters {
        println!("{}: {} tokens available", name, limiter.tokens_available());
    }
    
    assert!(final_successes > 0);
    assert!(final_failures > 0);
}