// Rate limiter using lock-free token bucket algorithm
//
// Uses a single AtomicU64 to store both token count and timestamp for
// maximum performance under high concurrency.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Error returned when rate limit is exceeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitExceeded;

impl std::fmt::Display for RateLimitExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rate limit exceeded")
    }
}

impl std::error::Error for RateLimitExceeded {}

/// Lock-free rate limiter using token bucket algorithm with hard limit (no burst)
pub struct RateLimiter {
    tokens_per_second: u32,
    // Single atomic storing packed state:
    // High 32 bits: available tokens
    // Low 32 bits: last refill timestamp (seconds since startup)
    state: AtomicU64,
    // Startup time for calculating relative timestamps
    startup_time: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with specified tokens per second
    pub fn new(tokens_per_second: u32) -> Self {
        let startup_time = Instant::now();
        Self {
            tokens_per_second,
            state: AtomicU64::new(0), // Start with 0 tokens to enforce limit from beginning
            startup_time,
        }
    }

    /// Try to acquire a token, returning error if rate limit is exceeded
    pub fn try_acquire_token(&self) -> Result<(), RateLimitExceeded> {
        let now = self.get_current_time_seconds();

        loop {
            let current_state = self.state.load(Ordering::Acquire);
            let (current_tokens, last_refill_time) = unpack_state(current_state);

            // Calculate refill using integer math only
            let time_diff = now.saturating_sub(last_refill_time);
            let tokens_to_add = time_diff.saturating_mul(self.tokens_per_second);

            // Hard limit: never allow more tokens than tokens_per_second (no burst)
            let available_tokens =
                (current_tokens.saturating_add(tokens_to_add)).min(self.tokens_per_second);

            if available_tokens == 0 {
                return Err(RateLimitExceeded);
            }

            // Atomically: refill + consume 1 token + update timestamp
            let new_tokens = available_tokens - 1;
            let new_state = pack_state(new_tokens, now);

            if self
                .state
                .compare_exchange_weak(
                    current_state,
                    new_state,
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Ok(());
            }
            // CAS failed, retry with updated values
        }
    }

    /// Check if a token is available without consuming it
    pub fn tokens_available(&self) -> u32 {
        let now = self.get_current_time_seconds();
        let current_state = self.state.load(Ordering::Acquire);
        let (current_tokens, last_refill_time) = unpack_state(current_state);

        let time_diff = now.saturating_sub(last_refill_time);
        let tokens_to_add = time_diff.saturating_mul(self.tokens_per_second);

        (current_tokens.saturating_add(tokens_to_add)).min(self.tokens_per_second)
    }

    /// Get configured tokens per second
    pub fn tokens_per_second(&self) -> u32 {
        self.tokens_per_second
    }

    /// Get current time in seconds since startup
    pub fn get_current_time_seconds(&self) -> u32 {
        let elapsed = self.startup_time.elapsed();
        elapsed.as_secs() as u32
    }
}

/// Pack token count and timestamp into a single u64
/// High 32 bits: tokens, Low 32 bits: timestamp
fn pack_state(tokens: u32, timestamp: u32) -> u64 {
    ((tokens as u64) << 32) | (timestamp as u64)
}

/// Unpack token count and timestamp from a u64
/// Returns (tokens, timestamp)
fn unpack_state(state: u64) -> (u32, u32) {
    let tokens = (state >> 32) as u32;
    let timestamp = state as u32;
    (tokens, timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_pack_unpack_state() {
        let tokens = 100;
        let timestamp = 1234567;
        let packed = pack_state(tokens, timestamp);
        let (unpacked_tokens, unpacked_timestamp) = unpack_state(packed);

        assert_eq!(tokens, unpacked_tokens);
        assert_eq!(timestamp, unpacked_timestamp);
    }

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(2); // 2 tokens per second

        // Should start with 0 tokens
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new(2); // 2 tokens per second

        // Wait for tokens to be added
        thread::sleep(Duration::from_millis(1100)); // > 1 second

        // Should now have tokens available
        assert!(limiter.tokens_available() >= 2);
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_rate_limiter_hard_limit() {
        let limiter = RateLimiter::new(5); // 5 tokens per second

        // Wait for tokens to accumulate
        thread::sleep(Duration::from_millis(3100)); // > 3 seconds

        // Should have at most 5 tokens (hard limit, no burst)
        assert!(limiter.tokens_available() <= 5);

        // Should be able to consume exactly 5 tokens
        for _ in 0..5 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // 6th token should fail
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_concurrent_access() {
        const NUM_THREADS: usize = 50;
        const TPS: u32 = 100;
        let limiter = Arc::new(RateLimiter::new(TPS));
        let barrier = Arc::new(Barrier::new(NUM_THREADS));
        let mut handles = vec![];

        // Wait for initial tokens to become available.
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), TPS);

        // Spawn threads that will all wait at the barrier.
        for _ in 0..NUM_THREADS {
            let limiter_clone = Arc::clone(&limiter);
            let barrier_clone = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                barrier_clone.wait(); // Synchronize all threads to start at once.
                let mut successes = 0;
                for _ in 0..10 {
                    if limiter_clone.try_acquire_token().is_ok() {
                        successes += 1;
                    }
                    // Small sleep to simulate work and allow other threads to run.
                    thread::sleep(Duration::from_millis(1));
                }
                successes
            });
            handles.push(handle);
        }

        // Collect results.
        let mut total_successes = 0;
        for handle in handles {
            total_successes += handle.join().unwrap();
        }

        println!("Concurrent test: {} successes", total_successes);

        // The number of successes should be exactly the number of available tokens.
        assert_eq!(total_successes, TPS as u64);
    }

    #[test]
    fn test_rate_limiting_precision() {
        let limiter = RateLimiter::new(10); // 10 tokens per second

        // Wait for tokens to accumulate
        thread::sleep(Duration::from_millis(2100)); // > 2 seconds

        // Should have 10 tokens available (hard limit)
        let available = limiter.tokens_available();
        assert!(available <= 10, "Available tokens: {}", available);

        // Consume all available tokens quickly
        let mut consumed = 0;
        while limiter.try_acquire_token().is_ok() {
            consumed += 1;
            if consumed > 15 {
                // Safety check to prevent infinite loop
                break;
            }
        }

        println!("Consumed {} tokens", consumed);
        assert!(consumed <= 10, "Should not consume more than 10 tokens");
    }

    #[test]
    fn test_refill_across_second_boundary() {
        let limiter = RateLimiter::new(1);

        // Wait for the first token.
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 1);

        // Consume the token.
        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 0);

        // It should be rate-limited now.
        assert!(limiter.try_acquire_token().is_err());

        // Wait for the next second boundary to pass.
        thread::sleep(Duration::from_millis(1100));

        // A new token should now be available.
        assert_eq!(limiter.tokens_available(), 1);
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_zero_tps_limiter() {
        // A limiter with 0 TPS should never grant a token.
        let limiter = RateLimiter::new(0);

        // Even after waiting, it should have 0 tokens.
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_rapid_consumption_within_one_second() {
        let limiter = RateLimiter::new(3);

        // Wait for tokens to be available.
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 3);

        // Consume all tokens.
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Immediately trying again should fail as we are in the same second.
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_long_idle_period() {
        let limiter = RateLimiter::new(10);

        // Wait a moment to ensure we have a non-zero current time
        thread::sleep(Duration::from_millis(1100));

        // Simulate a very long idle time by manipulating the internal state.
        // Set the timestamp to 5000 seconds in the past (but ensure it's not negative)
        let now = limiter.get_current_time_seconds();
        let long_ago = if now >= 5000 { now - 5000 } else { 0 };
        let initial_state = pack_state(0, long_ago);
        limiter.state.store(initial_state, Ordering::Relaxed);

        // After a long time, the number of available tokens should be capped
        // at tokens_per_second, not a huge number.
        let available = limiter.tokens_available();
        assert_eq!(
            available, 10,
            "Tokens should be capped at the hard limit after a long idle period"
        );

        // Should be able to consume up to the limit.
        for _ in 0..10 {
            assert!(limiter.try_acquire_token().is_ok());
        }
        // The next one should fail.
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_tokens_available_is_consistent() {
        let limiter = RateLimiter::new(5);
        thread::sleep(Duration::from_millis(1100));

        let initial_available = limiter.tokens_available();
        assert_eq!(initial_available, 5);

        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 4);

        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 3);
    }
}
