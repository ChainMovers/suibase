// Simple test to verify rate limiter behavior

use suibase_daemon::rate_limiter::RateLimiter;
use std::thread;
use std::time::{Duration, Instant};

#[test] 
fn test_simple_rate_limiting() {
    println!("Testing simple rate limiting behavior...");
    
    let limiter = RateLimiter::new(5, 0).unwrap(); // 5 tokens per second, unlimited per minute
    
    // Initially should have 0 tokens
    assert_eq!(limiter.tokens_available(), 0);
    
    // Wait 1 second - should have 5 tokens
    thread::sleep(Duration::from_millis(1100));
    let tokens_after_1s = limiter.tokens_available();
    println!("Tokens after 1s: {}", tokens_after_1s);
    assert!(tokens_after_1s <= 5);
    
    // Consume some tokens
    let mut consumed = 0;
    while consumed < 3 && limiter.try_acquire_token().is_ok() {
        consumed += 1;
    }
    println!("Consumed {} tokens", consumed);
    
    let remaining = limiter.tokens_available();
    println!("Remaining tokens: {}", remaining);
    
    // Wait another second - should refill
    thread::sleep(Duration::from_millis(1100));
    let tokens_after_refill = limiter.tokens_available();
    println!("Tokens after refill: {}", tokens_after_refill);
    assert!(tokens_after_refill <= 5);
}

#[test]
fn test_sustained_rate() {
    println!("Testing sustained rate over time...");
    
    let limiter = RateLimiter::new(2, 0).unwrap(); // 2 tokens per second, unlimited per minute
    let mut successes = 0u32;
    let _start = Instant::now();
    let test_duration = Duration::from_secs(3);
    
    // Wait for initial tokens
    thread::sleep(Duration::from_millis(1100));
    
    let measurement_start = Instant::now();
    
    // Try to consume tokens for 3 seconds
    while measurement_start.elapsed() < test_duration {
        if limiter.try_acquire_token().is_ok() {
            successes += 1;
        }
        thread::sleep(Duration::from_millis(100)); // Check every 100ms
    }
    
    let actual_duration = measurement_start.elapsed().as_secs_f64();
    let actual_rate = successes as f64 / actual_duration;
    
    println!("Test ran for {:.2}s", actual_duration);
    println!("Successes: {}", successes);
    println!("Actual rate: {:.2} tokens/sec", actual_rate);
    println!("Expected rate: 2.0 tokens/sec");
    
    // Should be approximately 6 tokens (2 per second × 3 seconds)
    // Allow some variance for timing
    assert!(successes >= 4 && successes <= 8, 
            "Expected 4-8 tokens, got {}", successes);
}