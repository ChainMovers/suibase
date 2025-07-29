// Additional edge case tests for rate limiter validation and boundary conditions

use suibase_daemon::rate_limiter::RateLimiter;

#[test]
fn test_validation_edge_cases() {
    println!("=== Testing validation edge cases ===");

    // Test maximum valid values
    let max_qps_valid = RateLimiter::new(32767, 0);
    assert!(max_qps_valid.is_ok(), "32767 QPS should be valid");

    let max_qpm_valid = RateLimiter::new(0, 262143);
    assert!(max_qpm_valid.is_ok(), "262143 QPM should be valid");

    // Test values exceeding limits
    let qps_too_large = RateLimiter::new(32768, 0);
    assert!(qps_too_large.is_err(), "32768 QPS should be invalid");
    assert_eq!(
        qps_too_large.unwrap_err(),
        "max_per_secs exceeds 32,767 limit"
    );

    let qpm_too_large = RateLimiter::new(0, 262144);
    assert!(qpm_too_large.is_err(), "262144 QPM should be invalid");
    assert_eq!(
        qpm_too_large.unwrap_err(),
        "max_per_min exceeds 262,143 limit"
    );

    // Test much larger values
    let way_too_large_qps = RateLimiter::new(1000000, 0);
    assert!(way_too_large_qps.is_err(), "1M QPS should be invalid");

    let way_too_large_qpm = RateLimiter::new(0, 1000000);
    assert!(way_too_large_qpm.is_err(), "1M QPM should be invalid");

    // Test both limits too large
    let both_too_large = RateLimiter::new(50000, 300000);
    assert!(
        both_too_large.is_err(),
        "Both limits too large should be invalid"
    );

    println!("✅ Validation edge cases tests passed");
}

#[test]
fn test_boundary_value_behavior() {
    println!("=== Testing boundary value behavior ===");

    // Test values right at the boundary
    let max_qps = RateLimiter::new(32767, 0).unwrap();
    assert_eq!(
        max_qps.tokens_available(),
        32767,
        "Max QPS should provide all tokens initially"
    );

    // Consume one token
    assert!(
        max_qps.try_acquire_token().is_ok(),
        "Should be able to consume one token"
    );
    assert_eq!(
        max_qps.tokens_available(),
        32766,
        "Should have one less token"
    );

    let max_qpm = RateLimiter::new(0, 262143).unwrap();
    assert_eq!(
        max_qpm.tokens_available(),
        262143,
        "Max QPM should provide all tokens initially"
    );

    // Consume one token
    assert!(
        max_qpm.try_acquire_token().is_ok(),
        "Should be able to consume one token"
    );
    assert_eq!(
        max_qpm.tokens_available(),
        262142,
        "Should have one less token"
    );

    // Test one below the boundary (should work)
    let just_under_qps = RateLimiter::new(32766, 0).unwrap();
    assert_eq!(
        just_under_qps.tokens_available(),
        32766,
        "32766 QPS should work"
    );

    let just_under_qpm = RateLimiter::new(0, 262142).unwrap();
    assert_eq!(
        just_under_qpm.tokens_available(),
        262142,
        "262142 QPM should work"
    );

    println!("✅ Boundary value behavior tests passed");
}

#[test]
fn test_mixed_success_failure_scenarios() {
    println!("=== Testing mixed success/failure scenarios ===");

    // Create limiter with small capacity
    let limiter = RateLimiter::new(3, 5).unwrap();
    assert_eq!(
        limiter.tokens_available(),
        3,
        "Should start with 3 tokens (limited by QPS)"
    );

    // Test partial consumption
    assert!(
        limiter.try_acquire_token().is_ok(),
        "First token should succeed"
    );
    assert_eq!(limiter.tokens_available(), 2, "Should have 2 tokens left");

    assert!(
        limiter.try_acquire_token().is_ok(),
        "Second token should succeed"
    );
    assert_eq!(limiter.tokens_available(), 1, "Should have 1 token left");

    assert!(
        limiter.try_acquire_token().is_ok(),
        "Third token should succeed"
    );
    assert_eq!(limiter.tokens_available(), 0, "Should be exhausted");

    // Now should fail
    assert!(
        limiter.try_acquire_token().is_err(),
        "Fourth token should fail"
    );
    assert_eq!(limiter.tokens_available(), 0, "Should still be exhausted");

    // Multiple failures in a row
    for i in 0..5 {
        assert!(
            limiter.try_acquire_token().is_err(),
            "Attempt {} should fail",
            i + 1
        );
        assert_eq!(limiter.tokens_available(), 0, "Should remain exhausted");
    }

    println!("✅ Mixed success/failure scenarios tests passed");
}

#[test]
fn test_time_sensitive_initialization() {
    println!("=== Testing time-sensitive initialization ===");

    use std::thread;
    use std::time::{Duration, Instant};

    // Test immediate availability vs. delayed availability
    let limiter = RateLimiter::new(5, 0).unwrap();

    // Should work immediately
    let start = Instant::now();
    assert!(
        limiter.try_acquire_token().is_ok(),
        "Should work immediately after creation"
    );
    let immediate_duration = start.elapsed();
    assert!(
        immediate_duration < Duration::from_millis(1),
        "Should be nearly instantaneous"
    );

    // Consume remaining tokens
    for _ in 0..4 {
        assert!(
            limiter.try_acquire_token().is_ok(),
            "Should consume remaining initial tokens"
        );
    }
    assert_eq!(limiter.tokens_available(), 0, "Should be exhausted");

    // Wait a small amount (less than 1 second)
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        limiter.tokens_available(),
        0,
        "Should still be exhausted after 100ms"
    );
    assert!(
        limiter.try_acquire_token().is_err(),
        "Should still fail after 100ms"
    );

    // Wait for refill
    thread::sleep(Duration::from_millis(1100));
    assert!(
        limiter.tokens_available() > 0,
        "Should have tokens after 1+ second"
    );
    assert!(
        limiter.try_acquire_token().is_ok(),
        "Should work after refill"
    );

    println!("✅ Time-sensitive initialization tests passed");
}

#[test]
fn test_zero_and_one_edge_cases() {
    println!("=== Testing zero and one edge cases ===");

    // Test (0, 1) - unlimited QPS, 1 QPM
    let unlimited_qps_one_qpm = RateLimiter::new(0, 1).unwrap();
    assert_eq!(
        unlimited_qps_one_qpm.tokens_available(),
        1,
        "(0,1) should start with 1 token"
    );
    assert!(
        unlimited_qps_one_qpm.try_acquire_token().is_ok(),
        "Should consume the one token"
    );
    assert_eq!(
        unlimited_qps_one_qpm.tokens_available(),
        0,
        "Should be exhausted"
    );
    assert!(
        unlimited_qps_one_qpm.try_acquire_token().is_err(),
        "Should fail when exhausted"
    );

    // Test (1, 0) - 1 QPS, unlimited QPM
    let one_qps_unlimited_qpm = RateLimiter::new(1, 0).unwrap();
    assert_eq!(
        one_qps_unlimited_qpm.tokens_available(),
        1,
        "(1,0) should start with 1 token"
    );
    assert!(
        one_qps_unlimited_qpm.try_acquire_token().is_ok(),
        "Should consume the one token"
    );
    assert_eq!(
        one_qps_unlimited_qpm.tokens_available(),
        0,
        "Should be exhausted"
    );
    assert!(
        one_qps_unlimited_qpm.try_acquire_token().is_err(),
        "Should fail when exhausted"
    );

    // Test (1, 1) - 1 QPS, 1 QPM
    let one_both = RateLimiter::new(1, 1).unwrap();
    assert_eq!(
        one_both.tokens_available(),
        1,
        "(1,1) should start with 1 token"
    );
    assert!(
        one_both.try_acquire_token().is_ok(),
        "Should consume the one token"
    );
    assert_eq!(one_both.tokens_available(), 0, "Should be exhausted");
    assert!(
        one_both.try_acquire_token().is_err(),
        "Should fail when exhausted"
    );

    println!("✅ Zero and one edge cases tests passed");
}

#[test]
fn test_large_scale_initial_consumption() {
    println!("=== Testing large-scale initial consumption ===");

    // Test with large initial capacity
    let large_limiter = RateLimiter::new(1000, 0).unwrap();
    assert_eq!(
        large_limiter.tokens_available(),
        1000,
        "Should start with 1000 tokens"
    );

    // Consume many tokens rapidly
    let mut consumed = 0;
    for i in 0..1000 {
        if large_limiter.try_acquire_token().is_ok() {
            consumed += 1;
        } else {
            panic!("Token acquisition {} failed unexpectedly", i + 1);
        }

        // Check that tokens_available is consistent
        let expected_remaining = 1000 - consumed;
        assert_eq!(
            large_limiter.tokens_available(),
            expected_remaining,
            "After consuming {} tokens, should have {} remaining",
            consumed,
            expected_remaining
        );
    }

    assert_eq!(consumed, 1000, "Should have consumed exactly 1000 tokens");
    assert_eq!(
        large_limiter.tokens_available(),
        0,
        "Should be fully exhausted"
    );
    assert!(
        large_limiter.try_acquire_token().is_err(),
        "Should fail when fully exhausted"
    );

    println!("✅ Large-scale initial consumption tests passed");
}
