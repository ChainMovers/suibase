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
/// Supports both QPS and QPM limits simultaneously using a unified atomic operation
#[derive(Debug)]
pub struct RateLimiter {
    max_per_secs: u32, // 0 = unlimited, max 32,767 (15 bits)
    max_per_min: u32,  // 0 = unlimited, max 262,143 (18 bits)
    // Single atomic storing packed state (64 bits):
    // [minute_window_id: 15][minute_tokens: 18][second_window_id: 16][second_tokens: 15]
    state: AtomicU64,
    startup_time: Instant,
}

impl RateLimiter {
    /// Create a rate limiter with both QPS and QPM limits
    /// max_per_secs: 0 = unlimited, >0 = tokens per second limit
    /// max_per_min: 0 = unlimited, >0 = tokens per minute limit
    /// Returns error if limits exceed bit field capacity
    pub fn new(max_per_secs: u32, max_per_min: u32) -> Result<Self, &'static str> {
        // Validate limits fit in their bit fields
        if max_per_secs > 32_767 {
            // 15 bits
            return Err("max_per_secs exceeds 32,767 limit");
        }
        if max_per_min > 262_143 {
            // 18 bits
            return Err("max_per_min exceeds 262,143 limit");
        }

        let startup_time = Instant::now();

        // Initialize state with full tokens available for immediate use
        // This prevents the "first minute" bug where QPM limiters don't work initially
        let initial_sec_tokens = if max_per_secs == 0 {
            0x7FFF // Max value for 15 bits (32,767)
        } else {
            max_per_secs
        };
        let initial_min_tokens = if max_per_min == 0 {
            0x3FFFF // Max value for 18 bits (262,143)
        } else {
            max_per_min
        };

        // Initialize window IDs to 0 to match time calculation at startup
        // (elapsed time starts at 0)
        let initial_sec_id = 0u32;
        let initial_min_id = 0u32;

        let initial_state = pack_state(
            initial_min_id,
            initial_min_tokens,
            initial_sec_id,
            initial_sec_tokens,
        );

        Ok(Self {
            max_per_secs,
            max_per_min,
            state: AtomicU64::new(initial_state),
            startup_time,
        })
    }

    /// Try to acquire a token, returning error if rate limit is exceeded
    pub fn try_acquire_token(&self) -> Result<(), RateLimitExceeded> {
        let start_time_secs = self.get_current_time_seconds();
        const TIMEOUT_SECS: u32 = 2; // Prevent indefinite blocking in async context

        let mut current_time_secs = start_time_secs;

        loop {
            let current_second_id = current_time_secs & 0xFFFF; // 16 bits
            let current_minute_id = (current_time_secs / 60) & 0x7FFF; // 15 bits

            let current_state = self.state.load(Ordering::Acquire);
            let (last_min_id, last_min_tokens, last_sec_id, last_sec_tokens) =
                unpack_state(current_state);

            // Calculate intended token counts based on current time (prevents stale refill race)
            let sec_window_fresh = self.has_window_expired(current_second_id, last_sec_id, true);
            let min_window_fresh = self.has_window_expired(current_minute_id, last_min_id, false);

            // Fail fast: if no windows are fresh and we need tokens but have none
            if !sec_window_fresh && !min_window_fresh {
                if self.max_per_secs > 0 && last_sec_tokens == 0 {
                    return Err(RateLimitExceeded);
                }
                if self.max_per_min > 0 && last_min_tokens == 0 {
                    return Err(RateLimitExceeded);
                }
            }

            let mut available_sec_tokens = if self.max_per_secs == 0 {
                // Unlimited: keep existing count to track usage, but ensure we have tokens
                if sec_window_fresh {
                    0x7FFF // Max value for 15 bits (32,767)
                } else {
                    last_sec_tokens.max(1) // Ensure at least 1 token remains to allow request
                }
            } else if sec_window_fresh {
                self.max_per_secs // Window expired: full refill
            } else {
                last_sec_tokens // Same window: use current count
            };

            let mut available_min_tokens = if self.max_per_min == 0 {
                // Unlimited: keep existing count to track usage, but ensure we have tokens
                if min_window_fresh {
                    0x3FFFF // Max value for 18 bits (262,143)
                } else {
                    last_min_tokens.max(1) // Ensure at least 1 token remains to allow request
                }
            } else if min_window_fresh {
                self.max_per_min // Window expired: full refill
            } else {
                last_min_tokens // Same window: use current count
            };

            // Check availability and consume tokens atomically (prevents premature failure)
            if self.max_per_secs > 0 {
                if available_sec_tokens == 0 {
                    // No tokens available - fail fast
                    return Err(RateLimitExceeded);
                }
            }
            // Always consume second tokens for tracking (even when unlimited)
            if available_sec_tokens > 0 {
                available_sec_tokens -= 1;
            }

            if self.max_per_min > 0 {
                if available_min_tokens == 0 {
                    // No tokens available - fail fast
                    return Err(RateLimitExceeded);
                }
            }
            // Always consume minute tokens for tracking (even when unlimited)
            if available_min_tokens > 0 {
                available_min_tokens -= 1;
            }

            // Attempt atomic update with calculated tokens
            let new_state = pack_state(
                current_minute_id,
                available_min_tokens,
                current_second_id,
                available_sec_tokens,
            );

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
                // Success: both limits satisfied atomically
                return Ok(());
            }
            // CAS failed, retry with fresh state
            current_time_secs = self.get_current_time_seconds();
            // Timeout protection for tokio context
            if current_time_secs > start_time_secs + TIMEOUT_SECS {
                return Err(RateLimitExceeded);
            }
        }
    }

    /// Check available tokens for monitoring (returns min of both limits)
    pub fn tokens_available(&self) -> u32 {
        let current_time_secs = self.get_current_time_seconds();
        let current_second_id = current_time_secs & 0xFFFF;
        let current_minute_id = (current_time_secs / 60) & 0x7FFF;

        let current_state = self.state.load(Ordering::Acquire);
        let (last_min_id, last_min_tokens, last_sec_id, last_sec_tokens) =
            unpack_state(current_state);

        let sec_window_expired = self.has_window_expired(current_second_id, last_sec_id, true);
        let min_window_expired = self.has_window_expired(current_minute_id, last_min_id, false);

        // Calculate current token availability (non-consuming)
        let available_sec_tokens = if self.max_per_secs == 0 {
            u32::MAX // Unlimited
        } else if sec_window_expired {
            self.max_per_secs
        } else {
            last_sec_tokens
        };

        let available_min_tokens = if self.max_per_min == 0 {
            u32::MAX // Unlimited
        } else if min_window_expired {
            self.max_per_min
        } else {
            last_min_tokens
        };

        // Return the limiting factor
        available_sec_tokens.min(available_min_tokens)
    }

    /// Get configured max per second
    pub fn max_per_secs(&self) -> u32 {
        self.max_per_secs
    }

    /// Get configured max per minute
    pub fn max_per_min(&self) -> u32 {
        self.max_per_min
    }

    /// Calculate current QPS and QPM based on token consumption
    /// Returns (QPS, QPM) for both rate-limited and unlimited servers
    /// Returns (0, 0) if windows are stale (more than 1 second/minute since last activity)
    /// 
    /// This method should be called periodically by monitoring systems for self-healing.
    /// It automatically clears stale state to prevent wrap-around bugs where old activity
    /// could be incorrectly reported after 18+ hours (QPS) or 22+ days (QPM).
    pub fn get_current_qps_qpm(&self) -> (u32, u32) {
        let current_time_secs = self.get_current_time_seconds();
        let current_second_id = current_time_secs & 0xFFFF;
        let current_minute_id = (current_time_secs / 60) & 0x7FFF;
        
        let observed_state = self.state.load(Ordering::Acquire);
        let (last_min_id, last_min_tokens, last_sec_id, last_sec_tokens) = unpack_state(observed_state);
        
        // Calculate window differences accounting for wrap-around
        let sec_window_diff = if current_second_id >= last_sec_id {
            current_second_id - last_sec_id
        } else {
            // Handle wrap-around (65536 second cycle)
            (0x10000 - last_sec_id) + current_second_id
        };
        
        let min_window_diff = if current_minute_id >= last_min_id {
            current_minute_id - last_min_id
        } else {
            // Handle wrap-around (32768 minute cycle)
            (0x8000 - last_min_id) + current_minute_id
        };
        
        // Calculate initial token values
        let initial_sec_tokens = if self.max_per_secs == 0 { 0x7FFF } else { self.max_per_secs };
        let initial_min_tokens = if self.max_per_min == 0 { 0x3FFFF } else { self.max_per_min };
        
        // Check if we need to clear stale state
        let sec_stale_with_consumption = sec_window_diff > 1 && last_sec_tokens < initial_sec_tokens;
        let min_stale_with_consumption = min_window_diff > 1 && last_min_tokens < initial_min_tokens;
        
        if sec_stale_with_consumption || min_stale_with_consumption {
            // Attempt to clear stale state atomically
            let new_state = pack_state(current_minute_id, initial_min_tokens, 
                                      current_second_id, initial_sec_tokens);
            
            // Only update if state hasn't changed (prevents race with concurrent acquire_token)
            let _ = self.state.compare_exchange(
                observed_state,
                new_state,
                Ordering::Release,
                Ordering::Acquire
            );
            // Whether the update succeeded or not, return 0 for stale windows
            return (0, 0);
        }
        
        // Calculate QPS (tokens consumed in second window)
        let qps = if sec_window_diff > 1 {
            0  // Stale window
        } else {
            initial_sec_tokens.saturating_sub(last_sec_tokens)
        };
        
        // Calculate QPM (tokens consumed in minute window)
        let qpm = if min_window_diff > 1 {
            0  // Stale window
        } else {
            initial_min_tokens.saturating_sub(last_min_tokens)
        };
        
        (qps, qpm)
    }


    /// Determines if a new time window has started, correctly handling both normal time progression and timer wrap-around
    fn has_window_expired(&self, current_id: u32, last_id: u32, is_second_window: bool) -> bool {
        if current_id == last_id {
            return false; // Same window
        }

        let half_range = if is_second_window {
            32767 // Half of 65535 (16-bit max)
        } else {
            16383 // Half of 32767 (15-bit max)
        };

        let diff = current_id.wrapping_sub(last_id);

        // Handle normal progression and wrap-around
        // diff > 0 && diff <= half_range means forward progression
        // Large diff values indicate backward movement (stale data)
        diff > 0 && diff <= half_range
    }

    /// Get current time in seconds since startup
    pub fn get_current_time_seconds(&self) -> u32 {
        // This could be speed optimized with CLOCK_MONOTONIC_COARSE, but
        // starting at zero brings some clarity while debugging.
        let elapsed = self.startup_time.elapsed();
        elapsed.as_secs() as u32
    }
}

/// Pack 15-bit minute ID, 18-bit minute tokens, 16-bit second ID, 15-bit second tokens
fn pack_state(min_id: u32, min_tokens: u32, sec_id: u32, sec_tokens: u32) -> u64 {
    ((min_id as u64 & 0x7FFF) << 49) |      // 15 bits: minute window ID
    ((min_tokens as u64 & 0x3FFFF) << 31) | // 18 bits: minute tokens
    ((sec_id as u64 & 0xFFFF) << 15) |      // 16 bits: second window ID
    (sec_tokens as u64 & 0x7FFF) // 15 bits: second tokens
}

/// Unpack 64-bit state into four values with correct bit masks
/// Returns (min_id, min_tokens, sec_id, sec_tokens)
fn unpack_state(state: u64) -> (u32, u32, u32, u32) {
    let min_id = ((state >> 49) & 0x7FFF) as u32; // 15 bits
    let min_tokens = ((state >> 31) & 0x3FFFF) as u32; // 18 bits
    let sec_id = ((state >> 15) & 0xFFFF) as u32; // 16 bits
    let sec_tokens = (state & 0x7FFF) as u32; // 15 bits
    (min_id, min_tokens, sec_id, sec_tokens)
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
        let min_id = 12345u32; // 15 bits
        let min_tokens = 150000u32; // 18 bits
        let sec_id = 54321u32; // 16 bits
        let sec_tokens = 25000u32; // 15 bits

        let packed = pack_state(min_id, min_tokens, sec_id, sec_tokens);
        let (unpacked_min_id, unpacked_min_tokens, unpacked_sec_id, unpacked_sec_tokens) =
            unpack_state(packed);

        assert_eq!(min_id, unpacked_min_id);
        assert_eq!(min_tokens, unpacked_min_tokens);
        assert_eq!(sec_id, unpacked_sec_id);
        assert_eq!(sec_tokens, unpacked_sec_tokens);
    }

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(2, 0).unwrap(); // 2 tokens per second, unlimited per minute

        // Should start with 2 tokens immediately available (bug fix)
        assert_eq!(limiter.tokens_available(), 2);
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new(2, 0).unwrap(); // 2 tokens per second, unlimited per minute

        // Wait for tokens to be added
        thread::sleep(Duration::from_millis(1100)); // > 1 second

        // Should now have tokens available
        assert!(limiter.tokens_available() >= 2);
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_rate_limiter_hard_limit() {
        let limiter = RateLimiter::new(5, 0).unwrap(); // 5 tokens per second, unlimited per minute

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
        let limiter = Arc::new(RateLimiter::new(TPS, 0).unwrap());
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
        let limiter = RateLimiter::new(10, 0).unwrap(); // 10 tokens per second, unlimited per minute

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
        let limiter = RateLimiter::new(1, 0).unwrap();

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
    fn test_zero_unlimited_limiter() {
        // A limiter with 0 TPS and 0 TPM should allow unlimited tokens.
        let limiter = RateLimiter::new(0, 0).unwrap();

        // Should have unlimited tokens available.
        thread::sleep(Duration::from_millis(1100));
        assert!(limiter.tokens_available() >= 1000); // Should be very large (unlimited)
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_unlimited_limiter_tracks_qps_qpm() {
        // Test that unlimited rate limiters still track QPS/QPM
        let limiter = RateLimiter::new(0, 0).unwrap();

        // Wait for initial window
        thread::sleep(Duration::from_millis(1100));

        // Make some requests
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Should now show 3 QPS and 3 QPM (3 requests made)
        let (qps, qpm) = limiter.get_current_qps_qpm();
        assert_eq!(qps, 3);
        assert_eq!(qpm, 3);

        // Make many more requests to verify tracking continues
        for _ in 0..50 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // Should now show 53 QPS and QPM
        let (qps, qpm) = limiter.get_current_qps_qpm();
        assert_eq!(qps, 53);
        assert_eq!(qpm, 53);

        // Wait for next second
        thread::sleep(Duration::from_millis(1100));

        // Should show previous window's activity
        let (qps, _) = limiter.get_current_qps_qpm();
        assert_eq!(qps, 53);

        // Make some requests in new window
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Current window QPS should now be 2, QPM should be cumulative: 53 + 2 = 55
        let (qps, qpm) = limiter.get_current_qps_qpm();
        assert_eq!(qps, 2);
        assert_eq!(qpm, 55);
    }

    #[test]
    fn test_partial_unlimited_tracking() {
        // Test with unlimited QPS but limited QPM
        let limiter_unlimited_qps = RateLimiter::new(0, 100).unwrap();

        thread::sleep(Duration::from_millis(1100));

        // Make 10 requests
        for _ in 0..10 {
            assert!(limiter_unlimited_qps.try_acquire_token().is_ok());
        }

        // QPS should track usage even though it's unlimited
        let (qps, qpm) = limiter_unlimited_qps.get_current_qps_qpm();
        assert_eq!(qps, 10);
        assert_eq!(qpm, 10);

        // Test with limited QPS but unlimited QPM
        let limiter_unlimited_qpm = RateLimiter::new(10, 0).unwrap();

        thread::sleep(Duration::from_millis(1100));

        // Make 5 requests
        for _ in 0..5 {
            assert!(limiter_unlimited_qpm.try_acquire_token().is_ok());
        }

        // Both should track usage
        let (qps, qpm) = limiter_unlimited_qpm.get_current_qps_qpm();
        assert_eq!(qps, 5);
        assert_eq!(qpm, 5);
    }

    #[test]
    fn test_rapid_consumption_within_one_second() {
        let limiter = RateLimiter::new(3, 0).unwrap();

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
        let limiter = RateLimiter::new(10, 0).unwrap();

        // Wait a moment to ensure we have a non-zero current time
        thread::sleep(Duration::from_millis(1100));

        // Simulate a very long idle time by manipulating the internal state.
        // Set the timestamp to 5000 seconds in the past (but ensure it's not negative)
        let now = limiter.get_current_time_seconds();
        let long_ago = if now >= 5000 { now - 5000 } else { 0 };
        let long_ago_min = (long_ago / 60) & 0x7FFF;
        let long_ago_sec = long_ago & 0xFFFF;
        let initial_state = pack_state(long_ago_min, 0, long_ago_sec, 0);
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
        let limiter = RateLimiter::new(5, 0).unwrap();
        thread::sleep(Duration::from_millis(1100));

        let initial_available = limiter.tokens_available();
        assert_eq!(initial_available, 5);

        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 4);

        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 3);
    }

    // === Dual Rate Limiting Tests ===

    #[test]
    fn test_dual_rate_limiting_basic() {
        // 3 tokens per second, 5 tokens per minute
        let limiter = RateLimiter::new(3, 5).unwrap();

        // Should start with 3 tokens (limited by min of QPS and QPM) immediately available (bug fix)
        assert_eq!(limiter.tokens_available(), 3);
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_dual_rate_limiting_qps_only() {
        // QPS only: 2 tokens per second, unlimited per minute
        let limiter = RateLimiter::new(2, 0).unwrap();

        // Wait for tokens to be available
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 2);

        // Should be able to consume 2 tokens
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Third token should fail
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_dual_rate_limiting_qpm_only() {
        // QPM only: unlimited per second, 3 tokens per minute
        let limiter = RateLimiter::new(0, 3).unwrap();

        // Wait for tokens to be available
        thread::sleep(Duration::from_millis(1100));

        // Should be limited by QPM (3), not QPS (unlimited)
        assert_eq!(limiter.tokens_available(), 3);

        // Should be able to consume 3 tokens
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Fourth token should fail (QPM limit reached)
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_dual_rate_limiting_both_limits() {
        // Both limits: 10 tokens per second, 15 tokens per minute
        let limiter = RateLimiter::new(10, 15).unwrap();

        // Should start with 10 tokens immediately available (limited by min of QPS and QPM) (bug fix)
        assert_eq!(limiter.tokens_available(), 10);

        // No need to wait - tokens are immediately available
        // Should be limited by the minimum of both (10 QPS)
        assert_eq!(limiter.tokens_available(), 10);

        // Consume 10 tokens rapidly
        for _ in 0..10 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // Should be exhausted now
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_dual_rate_limiting_qpm_more_restrictive() {
        // QPM is more restrictive: 60 tokens per second, 30 tokens per minute
        let limiter = RateLimiter::new(60, 30).unwrap();

        // Wait for initial window
        thread::sleep(Duration::from_millis(1100));

        // Should be limited by QPM (30), not QPS (60)
        assert_eq!(limiter.tokens_available(), 30);

        // Consume all 30 tokens
        for _ in 0..30 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // Should be exhausted
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_dual_rate_limiting_window_expiry() {
        // Test that windows expire correctly for both limits
        let limiter = RateLimiter::new(2, 3).unwrap();

        // Wait for tokens
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 2); // Limited by QPS

        // Consume both tokens
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 0);

        // Wait for next second
        thread::sleep(Duration::from_millis(1100));

        // Should have 1 more token (3 QPM - 2 already consumed = 1 remaining)
        assert_eq!(limiter.tokens_available(), 1);
        assert!(limiter.try_acquire_token().is_ok());

        // Now exhausted for this minute
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_dual_rate_limiting_configuration_validation() {
        // Test bit field limits
        assert!(RateLimiter::new(32_767, 262_143).is_ok()); // Max values
        assert!(RateLimiter::new(32_768, 0).is_err()); // QPS too large
        assert!(RateLimiter::new(0, 262_144).is_err()); // QPM too large
    }

    #[test]
    fn test_dual_rate_limiting_pack_unpack_stress() {
        // Test bit packing with various values
        let test_cases = vec![
            (0, 0, 0, 0),
            (0x7FFF, 0x3FFFF, 0xFFFF, 0x7FFF), // Max values
            (12345, 150000, 54321, 25000),     // Mid-range values
            (1, 1, 1, 1),                      // Min non-zero values
        ];

        for (min_id, min_tokens, sec_id, sec_tokens) in test_cases {
            let packed = pack_state(min_id, min_tokens, sec_id, sec_tokens);
            let (unpacked_min_id, unpacked_min_tokens, unpacked_sec_id, unpacked_sec_tokens) =
                unpack_state(packed);

            assert_eq!(
                min_id,
                unpacked_min_id,
                "min_id mismatch for {:?}",
                (min_id, min_tokens, sec_id, sec_tokens)
            );
            assert_eq!(
                min_tokens,
                unpacked_min_tokens,
                "min_tokens mismatch for {:?}",
                (min_id, min_tokens, sec_id, sec_tokens)
            );
            assert_eq!(
                sec_id,
                unpacked_sec_id,
                "sec_id mismatch for {:?}",
                (min_id, min_tokens, sec_id, sec_tokens)
            );
            assert_eq!(
                sec_tokens,
                unpacked_sec_tokens,
                "sec_tokens mismatch for {:?}",
                (min_id, min_tokens, sec_id, sec_tokens)
            );
        }
    }

    #[test]
    fn test_error_handling_rate_limit_exceeded() {
        // Test that RateLimitExceeded error can be properly formatted and debugged
        let error = RateLimitExceeded;
        assert_eq!(format!("{}", error), "Rate limit exceeded");
        assert_eq!(format!("{:?}", error), "RateLimitExceeded");

        // Test error trait implementation
        use std::error::Error;
        assert!(error.source().is_none());
    }

    #[test]
    fn test_zero_qps_with_nonzero_qpm() {
        // Test unlimited QPS with limited QPM - should be limited by QPM only
        let limiter = RateLimiter::new(0, 5).unwrap();

        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 5);

        // Should be able to consume all 5 QPM tokens instantly (unlimited QPS)
        for _ in 0..5 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // Sixth token should fail
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_nonzero_qps_with_zero_qpm() {
        // Test limited QPS with unlimited QPM - should be limited by QPS only
        let limiter = RateLimiter::new(3, 0).unwrap();

        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 3);

        // Should be able to consume 3 QPS tokens
        for _ in 0..3 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // Fourth token should fail (QPS limit)
        assert!(limiter.try_acquire_token().is_err());

        // Wait for next second
        thread::sleep(Duration::from_millis(1100));

        // Should have 3 more tokens available
        assert_eq!(limiter.tokens_available(), 3);
    }

    #[test]
    fn test_window_progression_detection() {
        // Test that window ID progression is handled correctly
        let limiter = RateLimiter::new(10, 20).unwrap();

        // Test normal forward progression
        assert!(limiter.has_window_expired(1, 0, true)); // Window 0 -> 1 (initial state special case)
        assert!(limiter.has_window_expired(100, 99, true)); // Window 99 -> 100
        assert!(limiter.has_window_expired(10, 5, true)); // Window 5 -> 10
        assert!(limiter.has_window_expired(1000, 999, false)); // Window 999 -> 1000

        // Test same window (no progression)
        assert!(!limiter.has_window_expired(5, 5, true)); // Same window
        assert!(!limiter.has_window_expired(100, 100, false)); // Same window

        // Test backward movement (should be detected as stale and ignored)
        assert!(!limiter.has_window_expired(50, 100, true)); // Backward: 100 -> 50
        assert!(!limiter.has_window_expired(1000, 2000, false)); // Backward: 2000 -> 1000

        // Test large forward jumps (should be detected as stale and ignored)
        assert!(!limiter.has_window_expired(50000, 1, true)); // Too large a jump
        assert!(!limiter.has_window_expired(20000, 1, false)); // Too large a jump

        // Test boundary cases for the half-range detection (using non-zero last_id to avoid special case)
        assert!(limiter.has_window_expired(32768, 1, true)); // Max valid forward jump for 16-bit: diff = 32767
        assert!(!limiter.has_window_expired(32769, 1, true)); // Just over the limit: diff = 32768
        assert!(limiter.has_window_expired(16384, 1, false)); // Max valid forward jump for 15-bit: diff = 16383
        assert!(!limiter.has_window_expired(16385, 1, false)); // Just over the limit: diff = 16384
    }

    #[test]
    fn test_immediate_failure_behavior() {
        // Test that try_acquire_token fails immediately when no tokens available in same window
        let limiter = RateLimiter::new(1, 1).unwrap();

        // Wait for tokens to be available
        thread::sleep(Duration::from_millis(1100));

        // Verify tokens are available first
        assert_eq!(limiter.tokens_available(), 1);

        // Consume the available tokens (both QPS and QPM)
        assert!(limiter.try_acquire_token().is_ok());

        // Verify no tokens remain
        assert_eq!(limiter.tokens_available(), 0);

        // Next attempt should fail immediately (quick check optimization)
        let start = Instant::now();
        let result = limiter.try_acquire_token();
        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(elapsed < Duration::from_millis(10)); // Should be very fast
    }

    #[test]
    fn test_tokens_available_never_exceeds_limits() {
        // Test that tokens_available never exceeds configured limits even after long waits
        let limiter = RateLimiter::new(5, 10).unwrap();

        // Wait much longer than necessary
        thread::sleep(Duration::from_millis(5100)); // 5+ seconds

        // Should still be capped at min(5, 10) = 5
        let available = limiter.tokens_available();
        assert!(
            available <= 5,
            "Available tokens {} exceeds QPS limit",
            available
        );
        assert_eq!(available, 5);
    }

    #[test]
    fn test_bit_field_boundary_values() {
        // Test that we can handle maximum values for each bit field

        // Test maximum valid configuration
        let limiter = RateLimiter::new(32_767, 262_143).unwrap();
        assert_eq!(limiter.max_per_secs(), 32_767);
        assert_eq!(limiter.max_per_min(), 262_143);

        // Test that values exceeding bit field limits are rejected
        assert!(RateLimiter::new(32_768, 0).is_err()); // QPS too large (16 bits: 32768 > 32767)
        assert!(RateLimiter::new(0, 262_144).is_err()); // QPM too large (18 bits: 262144 > 262143)

        // Test edge case: exactly at the boundary
        assert!(RateLimiter::new(32_767, 0).is_ok());
        assert!(RateLimiter::new(0, 262_143).is_ok());
    }

    #[test]
    fn test_state_consistency_after_operations() {
        // Test that internal state remains consistent after various operations
        let limiter = RateLimiter::new(3, 5).unwrap();

        thread::sleep(Duration::from_millis(1100));

        // Consume some tokens
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Verify state consistency
        let available = limiter.tokens_available();
        assert_eq!(available, 1); // min(3-2, 5-2) = min(1, 3) = 1

        // Consume remaining token
        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 0);

        // Should be rate limited now
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_initial_state_edge_cases() {
        // Test various scenarios at the very beginning of rate limiter lifecycle

        // Test that tokens_available() works correctly at time 0 (bug fix)
        let limiter = RateLimiter::new(10, 15).unwrap();
        assert_eq!(limiter.tokens_available(), 10); // Should start with 10 tokens (limited by min)

        // Test that try_acquire_token succeeds immediately at startup (bug fix)
        assert!(limiter.try_acquire_token().is_ok());

        // Test unlimited limiter at startup
        let unlimited = RateLimiter::new(0, 0).unwrap();
        assert!(unlimited.tokens_available() >= 1000); // Should be unlimited
        assert!(unlimited.try_acquire_token().is_ok());
    }

    #[test]
    fn test_rapid_successive_window_transitions() {
        // Test behavior during rapid window transitions
        let limiter = RateLimiter::new(1, 2).unwrap();

        // Wait for initial tokens
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 1); // min(1, 2) = 1

        // Consume token
        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 0);

        // Wait for next second
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 1); // min(1, 1) = 1 (remaining QPM)

        // Consume final QPM token
        assert!(limiter.try_acquire_token().is_ok());
        assert_eq!(limiter.tokens_available(), 0);

        // Should be exhausted for this minute
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_bit_packing_boundaries() {
        // Test with maximum values for each field to ensure no bit overflow or bleed.
        let max_min_id = 0x7FFF; // 15 bits
        let max_min_tokens = 0x3FFFF; // 18 bits
        let max_sec_id = 0xFFFF; // 16 bits
        let max_sec_tokens = 0x7FFF; // 15 bits

        let packed = pack_state(max_min_id, max_min_tokens, max_sec_id, max_sec_tokens);
        let (unpacked_min_id, unpacked_min_tokens, unpacked_sec_id, unpacked_sec_tokens) =
            unpack_state(packed);

        assert_eq!(unpacked_min_id, max_min_id);
        assert_eq!(unpacked_min_tokens, max_min_tokens);
        assert_eq!(unpacked_sec_id, max_sec_id);
        assert_eq!(unpacked_sec_tokens, max_sec_tokens);
    }

    #[test]
    fn test_qps_qpm_calculation() {
        // Test QPS calculation
        let limiter = RateLimiter::new(10, 100).unwrap();

        // Initially should show 0 since no tokens consumed
        let (qps, qpm) = limiter.get_current_qps_qpm();
        assert_eq!(qps, 0);
        assert_eq!(qpm, 0);

        // Wait for initial tokens to be available
        thread::sleep(Duration::from_millis(1100));

        // Consume some tokens
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Should show 3 QPS (3 tokens consumed from 10 limit) and 3 QPM
        let (qps, qpm) = limiter.get_current_qps_qpm();
        assert_eq!(qps, 3);
        assert_eq!(qpm, 3);
    }

    #[test]
    fn test_qps_qpm_with_unlimited() {
        // Test with unlimited limits - now tracks actual usage
        let unlimited_qps = RateLimiter::new(0, 100).unwrap();
        let unlimited_qpm = RateLimiter::new(10, 0).unwrap();
        let both_unlimited = RateLimiter::new(0, 0).unwrap();

        thread::sleep(Duration::from_millis(1100));

        // Consume tokens
        assert!(unlimited_qps.try_acquire_token().is_ok());
        assert!(unlimited_qpm.try_acquire_token().is_ok());
        assert!(both_unlimited.try_acquire_token().is_ok());

        // Unlimited QPS (0 limit) should track actual usage: 1 QPS and 1 QPM
        let (qps, qpm) = unlimited_qps.get_current_qps_qpm();
        assert_eq!(qps, 1);
        assert_eq!(qpm, 1);

        // Unlimited QPM (0 limit) should track actual usage: 1 QPS and 1 QPM
        let (qps, qpm) = unlimited_qpm.get_current_qps_qpm();
        assert_eq!(qps, 1);
        assert_eq!(qpm, 1);

        // Both unlimited should track actual usage: 1 for both
        let (qps, qpm) = both_unlimited.get_current_qps_qpm();
        assert_eq!(qps, 1);
        assert_eq!(qpm, 1);
    }

    #[test]
    fn test_bit_packing_zeroes_and_ones() {
        // Test with zero to ensure it packs and unpacks correctly.
        let packed_zero = pack_state(0, 0, 0, 0);
        assert_eq!(packed_zero, 0);
        let (min_id, min_tokens, sec_id, sec_tokens) = unpack_state(packed_zero);
        assert_eq!((min_id, min_tokens, sec_id, sec_tokens), (0, 0, 0, 0));

        // Test with one in each field to check for correct bit isolation.
        let packed_one = pack_state(1, 1, 1, 1);
        let (min_id, min_tokens, sec_id, sec_tokens) = unpack_state(packed_one);
        assert_eq!((min_id, min_tokens, sec_id, sec_tokens), (1, 1, 1, 1));
    }

    #[test]
    fn test_refill_at_precise_second_boundary() {
        // Verifies that tokens are not available just before the window ends,
        // but are available immediately after.
        let limiter = RateLimiter::new(1, 0).unwrap(); // 1 QPS, no QPM limit

        // Wait for initial token to be available
        thread::sleep(Duration::from_millis(1100));

        // Consume the only token.
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_err());

        // Sleep until just before the next second boundary.
        let elapsed = limiter.startup_time.elapsed();
        let next_second = ((elapsed.as_secs() + 1) * 1000) as u64;
        let current_millis = elapsed.as_millis() as u64;
        let until_next_sec = next_second.saturating_sub(current_millis);

        if until_next_sec > 50 {
            thread::sleep(Duration::from_millis(until_next_sec - 50));
            // Should still be rate-limited.
            assert!(limiter.try_acquire_token().is_err());
        }

        // Sleep past the boundary.
        thread::sleep(Duration::from_millis(100));

        // Should now be refilled and successful.
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_dual_refill_at_minute_boundary() {
        // Ensure that when a minute window expires, BOTH the minute and second
        // buckets are refilled correctly.
        let limiter = RateLimiter::new(10, 100).unwrap(); // 10 QPS, 100 QPM

        // Wait for initial window
        thread::sleep(Duration::from_millis(1100));

        // Consume most QPS tokens to create interesting state
        for _ in 0..10 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // At this point, QPS is exhausted but QPM still has tokens
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());

        // Sleep for 1.1 seconds to cross the second boundary.
        thread::sleep(Duration::from_millis(1100));

        // After the second boundary, QPS should be refilled. The limiting factor
        // is now the QPS limit again.
        assert_eq!(limiter.tokens_available(), 10);
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_concurrent_storm_at_window_boundary() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};

        // Spawn more threads than available tokens and have them all try to acquire
        // at the exact same moment using a barrier.
        let limiter = Arc::new(RateLimiter::new(10, 0).unwrap()); // 10 QPS
        let num_threads = 50;
        let barrier = Arc::new(Barrier::new(num_threads));
        let success_count = Arc::new(AtomicUsize::new(0));

        // Wait for initial tokens to be available
        thread::sleep(Duration::from_millis(1100));

        let mut handles = vec![];
        for _ in 0..num_threads {
            let limiter_clone = Arc::clone(&limiter);
            let barrier_clone = Arc::clone(&barrier);
            let success_clone = Arc::clone(&success_count);

            handles.push(thread::spawn(move || {
                // All threads wait here until the last one arrives.
                barrier_clone.wait();
                // At this moment, all 50 threads will rush to acquire a token.
                if limiter_clone.try_acquire_token().is_ok() {
                    success_clone.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Exactly 10 threads should have succeeded. Any more or less indicates a race condition.
        assert_eq!(success_count.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_immediate_failure_vs_timeout_behavior() {
        let limiter = RateLimiter::new(1, 0).unwrap();

        // Wait for initial token
        thread::sleep(Duration::from_millis(1100));

        // 1. Consume the only token.
        assert!(limiter.try_acquire_token().is_ok());

        // 2. Immediately try again in the same window. This should fail immediately
        // because our optimized implementation detects when no windows are fresh
        // and no tokens are available.
        let now = Instant::now();
        assert_eq!(limiter.try_acquire_token(), Err(RateLimitExceeded));
        // Verify it failed quickly due to immediate check optimization
        assert!(now.elapsed() < Duration::from_millis(100));

        // 3. Sleep to advance the window, then try again.
        thread::sleep(Duration::from_millis(1100));

        // 4. Consume the newly refilled token.
        assert!(limiter.try_acquire_token().is_ok());

        // 5. Try again. This should also fail immediately since we're in the same window
        // and have consumed all tokens.
        let now = Instant::now();
        assert_eq!(limiter.try_acquire_token(), Err(RateLimitExceeded));
        // Verify it failed quickly
        assert!(now.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn test_wait_for_window_transition() {
        // Test a scenario where the algorithm waits for a window transition
        let limiter = RateLimiter::new(2, 3).unwrap(); // 2 QPS, 3 QPM

        // Wait for initial tokens
        thread::sleep(Duration::from_millis(1100));

        // Consume all QPS tokens but leave QPM tokens available
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Now we're QPS-limited but QPM still has capacity
        assert_eq!(limiter.tokens_available(), 0);

        // This should fail immediately because same window, no QPS tokens
        let now = Instant::now();
        assert_eq!(limiter.try_acquire_token(), Err(RateLimitExceeded));
        assert!(now.elapsed() < Duration::from_millis(100));

        // After waiting for next second, should get QPS refill
        thread::sleep(Duration::from_millis(1100));
        assert_eq!(limiter.tokens_available(), 1); // min(2, 1) = 1 (remaining QPM)
        assert!(limiter.try_acquire_token().is_ok());
    }

    #[test]
    fn test_tokens_available_accuracy() {
        // Test that tokens_available() explicitly returns accurate values
        // without consuming tokens
        let limiter = RateLimiter::new(5, 12).unwrap(); // 5 QPS, 12 QPM

        // Wait for initial tokens
        thread::sleep(Duration::from_millis(1100));

        // Initially should return min(5, 12) = 5
        assert_eq!(limiter.tokens_available(), 5);

        // Consume 3 tokens
        for _ in 0..3 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // Should now return min(2, 9) = 2
        assert_eq!(limiter.tokens_available(), 2);

        // Multiple calls to tokens_available() should not change the result
        assert_eq!(limiter.tokens_available(), 2);
        assert_eq!(limiter.tokens_available(), 2);

        // Consume remaining QPS tokens
        assert!(limiter.try_acquire_token().is_ok());
        assert!(limiter.try_acquire_token().is_ok());

        // Should now be QPS-limited: min(0, 7) = 0
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());
    }

    #[test]
    fn test_qpm_as_bottleneck() {
        // Test scenario where QPM is the limiting factor
        let limiter = RateLimiter::new(10, 5).unwrap(); // 10 QPS, 5 QPM (QPM more restrictive)

        // Wait for initial tokens
        thread::sleep(Duration::from_millis(1100));

        // Should be QPM-limited: min(10, 5) = 5
        assert_eq!(limiter.tokens_available(), 5);

        // Consume all QPM tokens
        for _ in 0..5 {
            assert!(limiter.try_acquire_token().is_ok());
        }

        // Should now be QPM-exhausted: min(5, 0) = 0
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());

        // Wait for next second (QPS refills but QPM doesn't)
        thread::sleep(Duration::from_millis(1100));

        // Still QPM-limited: min(10, 0) = 0
        // This validates that QPM is correctly enforced as the bottleneck
        assert_eq!(limiter.tokens_available(), 0);
        assert!(limiter.try_acquire_token().is_err());

        // Test that the limiting logic works: QPS has tokens but QPM doesn't
        // This confirms the min() logic in tokens_available() is working correctly
        let current_state = limiter.state.load(std::sync::atomic::Ordering::Acquire);
        let (_, min_tokens, _, _sec_tokens) = unpack_state(current_state);

        // QPS should have refilled (or be unlimited), QPM should be 0
        // The exact values depend on timing, but QPM should be the constraint
        assert_eq!(min_tokens, 0, "QPM tokens should be exhausted");

        // Verify one more time that QPM limit is enforced
        assert_eq!(limiter.tokens_available(), 0);
    }
}
