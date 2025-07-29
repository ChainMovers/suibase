// Comprehensive tests for rate limiter initial state correctness
// Validates that the initialization bug fix works across all edge cases

use std::thread;
use std::time::Duration;
use suibase_daemon::rate_limiter::RateLimiter;

#[test]
fn test_boundary_value_initialization() {
    println!("=== Testing boundary value initialization ===");

    // Maximum QPS value (32,767)
    let max_qps_limiter = RateLimiter::new(32767, 0).unwrap();
    assert_eq!(
        max_qps_limiter.tokens_available(),
        32767,
        "Max QPS should start with 32,767 tokens"
    );
    assert!(
        max_qps_limiter.try_acquire_token().is_ok(),
        "Max QPS should allow immediate acquisition"
    );
    assert_eq!(
        max_qps_limiter.tokens_available(),
        32766,
        "Should have one less token after acquisition"
    );

    // Maximum QPM value (262,143)
    let max_qpm_limiter = RateLimiter::new(0, 262143).unwrap();
    assert_eq!(
        max_qpm_limiter.tokens_available(),
        262143,
        "Max QPM should start with 262,143 tokens"
    );
    assert!(
        max_qpm_limiter.try_acquire_token().is_ok(),
        "Max QPM should allow immediate acquisition"
    );
    assert_eq!(
        max_qpm_limiter.tokens_available(),
        262142,
        "Should have one less token after acquisition"
    );

    // Both at maximum (should be limited by QPS)
    let max_both_limiter = RateLimiter::new(32767, 262143).unwrap();
    assert_eq!(
        max_both_limiter.tokens_available(),
        32767,
        "Dual max should be limited by QPS (smaller)"
    );

    println!("✅ Boundary value initialization tests passed");
}

#[test]
fn test_minimum_value_initialization() {
    println!("=== Testing minimum value initialization ===");

    // Minimum non-zero values
    let min_qps_limiter = RateLimiter::new(1, 0).unwrap();
    assert_eq!(
        min_qps_limiter.tokens_available(),
        1,
        "1 QPS should start with 1 token"
    );
    assert!(
        min_qps_limiter.try_acquire_token().is_ok(),
        "Should allow single token acquisition"
    );
    assert_eq!(
        min_qps_limiter.tokens_available(),
        0,
        "Should be exhausted after one token"
    );
    assert!(
        min_qps_limiter.try_acquire_token().is_err(),
        "Should fail when exhausted"
    );

    let min_qpm_limiter = RateLimiter::new(0, 1).unwrap();
    assert_eq!(
        min_qpm_limiter.tokens_available(),
        1,
        "1 QPM should start with 1 token"
    );
    assert!(
        min_qpm_limiter.try_acquire_token().is_ok(),
        "Should allow single token acquisition"
    );
    assert_eq!(
        min_qpm_limiter.tokens_available(),
        0,
        "Should be exhausted after one token"
    );
    assert!(
        min_qpm_limiter.try_acquire_token().is_err(),
        "Should fail when exhausted"
    );

    // Both at minimum (both should provide 1 token)
    let min_both_limiter = RateLimiter::new(1, 1).unwrap();
    assert_eq!(
        min_both_limiter.tokens_available(),
        1,
        "1 QPS + 1 QPM should start with 1 token"
    );
    assert!(
        min_both_limiter.try_acquire_token().is_ok(),
        "Should allow single token acquisition"
    );
    assert_eq!(
        min_both_limiter.tokens_available(),
        0,
        "Should be exhausted after one token"
    );

    println!("✅ Minimum value initialization tests passed");
}

#[test]
fn test_asymmetric_limits_initialization() {
    println!("=== Testing asymmetric limits initialization ===");

    // QPS much smaller than QPM
    let qps_limiting = RateLimiter::new(5, 1000).unwrap();
    assert_eq!(
        qps_limiting.tokens_available(),
        5,
        "Should be limited by smaller QPS value"
    );

    // QPM much smaller than QPS
    let qpm_limiting = RateLimiter::new(1000, 5).unwrap();
    assert_eq!(
        qpm_limiting.tokens_available(),
        5,
        "Should be limited by smaller QPM value"
    );

    // Extreme asymmetry (within limits)
    let extreme_qps = RateLimiter::new(1, 50000).unwrap();
    assert_eq!(
        extreme_qps.tokens_available(),
        1,
        "Extreme asymmetry should work correctly"
    );

    let extreme_qpm = RateLimiter::new(30000, 1).unwrap();
    assert_eq!(
        extreme_qpm.tokens_available(),
        1,
        "Extreme asymmetry should work correctly"
    );

    println!("✅ Asymmetric limits initialization tests passed");
}

#[test]
fn test_rapid_initial_consumption() {
    println!("=== Testing rapid initial token consumption ===");

    // Create limiter with 10 tokens and consume them rapidly
    let limiter = RateLimiter::new(10, 0).unwrap();
    assert_eq!(
        limiter.tokens_available(),
        10,
        "Should start with 10 tokens"
    );

    // Consume all tokens rapidly
    for i in 0..10 {
        assert!(
            limiter.try_acquire_token().is_ok(),
            "Token {} should be available",
            i + 1
        );
        assert_eq!(
            limiter.tokens_available(),
            9 - i,
            "Should have {} tokens remaining",
            9 - i
        );
    }

    // Should be exhausted
    assert_eq!(limiter.tokens_available(), 0, "Should be exhausted");
    assert!(
        limiter.try_acquire_token().is_err(),
        "Should fail when exhausted"
    );

    println!("✅ Rapid initial consumption tests passed");
}

#[test]
fn test_zero_nonzero_combinations() {
    println!("=== Testing all zero/non-zero combinations ===");

    // (0, 0) - Unlimited
    let unlimited = RateLimiter::new(0, 0).unwrap();
    assert_eq!(
        unlimited.tokens_available(),
        u32::MAX,
        "Unlimited should have max tokens"
    );
    for _ in 0..100 {
        assert!(
            unlimited.try_acquire_token().is_ok(),
            "Unlimited should never fail"
        );
    }
    assert_eq!(
        unlimited.tokens_available(),
        u32::MAX,
        "Unlimited should still have max tokens"
    );

    // (N, 0) - QPS limited, QPM unlimited
    let qps_only = RateLimiter::new(7, 0).unwrap();
    assert_eq!(
        qps_only.tokens_available(),
        7,
        "QPS-only should start with QPS tokens"
    );

    // (0, N) - QPS unlimited, QPM limited
    let qpm_only = RateLimiter::new(0, 12).unwrap();
    assert_eq!(
        qpm_only.tokens_available(),
        12,
        "QPM-only should start with QPM tokens"
    );

    // (N, M) - Both limited
    let both_limited = RateLimiter::new(15, 8).unwrap();
    assert_eq!(
        both_limited.tokens_available(),
        8,
        "Should be limited by smaller value"
    );

    println!("✅ Zero/non-zero combination tests passed");
}

#[test]
fn test_initial_state_consistency() {
    println!("=== Testing initial state consistency ===");

    let test_cases = vec![
        (1, 0, 1),
        (0, 1, 1),
        (5, 0, 5),
        (0, 10, 10),
        (3, 7, 3),
        (8, 2, 2),
        (100, 200, 100),
        (500, 50, 50),
    ];

    for (qps, qpm, expected_tokens) in test_cases {
        let limiter = RateLimiter::new(qps, qpm).unwrap();

        // Check that tokens_available() and try_acquire_token() are consistent
        let available_before = limiter.tokens_available();
        assert_eq!(
            available_before, expected_tokens,
            "QPS={}, QPM={}: Expected {} tokens, got {}",
            qps, qpm, expected_tokens, available_before
        );

        if available_before > 0 {
            assert!(
                limiter.try_acquire_token().is_ok(),
                "QPS={}, QPM={}: Should be able to acquire token when {} available",
                qps,
                qpm,
                available_before
            );

            let available_after = limiter.tokens_available();
            assert_eq!(
                available_after,
                available_before - 1,
                "QPS={}, QPM={}: Should have one less token after acquisition",
                qps,
                qpm
            );
        } else {
            assert!(
                limiter.try_acquire_token().is_err(),
                "QPS={}, QPM={}: Should not be able to acquire token when none available",
                qps,
                qpm
            );
        }
    }

    println!("✅ Initial state consistency tests passed");
}

#[test]
fn test_immediate_exhaustion_and_refill() {
    println!("=== Testing immediate exhaustion and refill ===");

    // Create limiter with 3 tokens
    let limiter = RateLimiter::new(3, 0).unwrap();
    assert_eq!(limiter.tokens_available(), 3, "Should start with 3 tokens");

    // Exhaust all tokens immediately
    assert!(limiter.try_acquire_token().is_ok());
    assert!(limiter.try_acquire_token().is_ok());
    assert!(limiter.try_acquire_token().is_ok());
    assert_eq!(limiter.tokens_available(), 0, "Should be exhausted");
    assert!(
        limiter.try_acquire_token().is_err(),
        "Should fail when exhausted"
    );

    // Wait for refill
    thread::sleep(Duration::from_millis(1100));
    assert!(
        limiter.tokens_available() > 0,
        "Should have tokens after refill"
    );
    assert!(
        limiter.try_acquire_token().is_ok(),
        "Should work after refill"
    );

    println!("✅ Immediate exhaustion and refill tests passed");
}

#[test]
fn test_packed_state_correctness() {
    println!("=== Testing packed state correctness at initialization ===");

    // Test that the internal packed state is correctly initialized
    // We can't directly access the packed state, but we can verify behavior

    let limiter = RateLimiter::new(5, 10).unwrap();

    // Initial state should allow 5 acquisitions (limited by QPS)
    for i in 0..5 {
        assert!(
            limiter.try_acquire_token().is_ok(),
            "Acquisition {} should succeed",
            i + 1
        );
    }

    // Should be exhausted after 5 acquisitions
    assert_eq!(
        limiter.tokens_available(),
        0,
        "Should be exhausted after 5 acquisitions"
    );
    assert!(
        limiter.try_acquire_token().is_err(),
        "Should fail after exhaustion"
    );

    // Create another limiter to test QPM limiting
    let qpm_limited = RateLimiter::new(10, 3).unwrap();

    // Should allow 3 acquisitions (limited by QPM)
    for i in 0..3 {
        assert!(
            qpm_limited.try_acquire_token().is_ok(),
            "QPM acquisition {} should succeed",
            i + 1
        );
    }

    assert_eq!(
        qpm_limited.tokens_available(),
        0,
        "Should be exhausted after 3 acquisitions"
    );
    assert!(
        qpm_limited.try_acquire_token().is_err(),
        "Should fail after QPM exhaustion"
    );

    println!("✅ Packed state correctness tests passed");
}

#[test]
fn test_concurrent_initial_access() {
    println!("=== Testing concurrent access to initial tokens ===");

    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let limiter: Arc<RateLimiter> = Arc::new(RateLimiter::new(20, 0).unwrap());
    let success_count = Arc::new(AtomicU32::new(0));
    let total_attempts = 50;

    // Spawn multiple threads trying to acquire tokens immediately
    let mut handles = vec![];

    for _ in 0..total_attempts {
        let limiter_clone = Arc::clone(&limiter);
        let success_count_clone = Arc::clone(&success_count);

        let handle = std::thread::spawn(move || {
            if limiter_clone.try_acquire_token().is_ok() {
                success_count_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let successes = success_count.load(Ordering::Relaxed);

    // Should have exactly 20 successes (the initial token count)
    assert_eq!(
        successes, 20,
        "Should have exactly 20 successful acquisitions from initial tokens"
    );

    // Remaining attempts should fail
    assert_eq!(
        limiter.tokens_available(),
        0,
        "Should be exhausted after concurrent access"
    );

    println!(
        "✅ Concurrent initial access tests passed - {} successes out of {} attempts",
        successes, total_attempts
    );
}

#[test]
fn test_initialization_vs_refill_behavior() {
    println!("=== Testing initialization vs refill behavior ===");

    // Test that initialization behavior matches refill behavior
    let limiter1 = RateLimiter::new(5, 0).unwrap();
    let initial_tokens = limiter1.tokens_available();

    // Consume all initial tokens
    while limiter1.try_acquire_token().is_ok() {}

    // Wait for refill
    thread::sleep(Duration::from_millis(1100));
    let refill_tokens = limiter1.tokens_available();

    // Create a new limiter for comparison
    let limiter2 = RateLimiter::new(5, 0).unwrap();
    let new_initial_tokens = limiter2.tokens_available();

    assert_eq!(initial_tokens, 5, "Initial tokens should be 5");
    assert_eq!(refill_tokens, 5, "Refill tokens should be 5");
    assert_eq!(
        new_initial_tokens, 5,
        "New limiter should also start with 5"
    );
    assert_eq!(
        initial_tokens, refill_tokens,
        "Initial and refill behavior should match"
    );
    assert_eq!(
        initial_tokens, new_initial_tokens,
        "All limiters should behave consistently"
    );

    println!("✅ Initialization vs refill behavior tests passed");
}
