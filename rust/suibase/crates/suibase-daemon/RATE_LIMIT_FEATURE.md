# Rate Limiting Feature for Suibase-daemon

## Overview
Add configurable rate limiting to RPC links in suibase-daemon to prevent overwhelming upstream servers and enable fair distribution of requests across multiple servers.

## Requirements
1. **Configuration**: Rate limits defined in suibase.yaml per RPC server
2. **Server Selection**: Consider rate limits when selecting next RPC server
3. **Fallback Strategy**: Try alternative servers when one hits rate limit
4. **Blocking Behavior**: Block/delay requests when all servers are rate limited
5. **Thread Safety**: Handle concurrent requests without race conditions
6. **Performance**: Efficient tracking for high-throughput scenarios
7. **Statistics**: Track percentage of requests skipped due to rate limiting

## Design Considerations
- Number of RPC servers expected to be low (<100)
- Must integrate with existing server selection logic
- Statistics should be user-visible
- Rate limiting should be optional and backward compatible

## Current Architecture Analysis

### Server Selection Mechanism
- **ProxyServer** (`proxy_server.rs`): Handles incoming RPC requests
- **InputPort** (`input_port.rs`): Contains `get_best_target_servers()` which selects servers based on:
  - Health status (OK/DOWN)
  - Latency grouping (servers within 2x or 25% of best latency)
  - Load balancing within same quality tier using random selection
- **NetworkMonitor** (`network_monitor.rs`): Tracks server health and latency statistics
- **ServerStats** (`server_stats.rs`): Maintains per-server statistics

### Configuration Structure
- Links defined in `suibase.yaml` with fields: alias, rpc, ws, priority, selectable, monitored, max_per_secs
- Each workdir has its own configuration

## Proposed Design

### 1. Configuration Schema
Add rate limiting field to the Link configuration in `suibase.yaml`:

```yaml
links:
  - alias: "sui.io"
    rpc: "https://fullnode.testnet.sui.io:443"
    ws: "wss://fullnode.testnet.sui.io:443"
    priority: 10
    max_per_secs: 100  # Optional: maximum requests per second
```

### 2. Rate Limiter Architecture

#### Core Components
1. **RateLimiter struct** (new file: `rate_limiter.rs`)
   - Token bucket algorithm with hard limit (no bursts)
   - Per-server rate limiter instances
   - Thread-safe using atomic operations

```rust
struct RateLimiter {
    tokens_per_second: u32, // Use integer to avoid floating-point precision issues
    // Single atomic containing both tokens and timestamp (lock-free)
    // High 32 bits: available tokens, Low 32 bits: last refill timestamp (seconds)
    state: AtomicU64,
}

impl RateLimiter {
    fn try_acquire_token(&self) -> Result<(), RateLimitExceeded> {
        loop {
            let now = get_monotonic_time_u32(); // Recalculate time each iteration
            let current_state = self.state.load(Ordering::Acquire);
            let (current_tokens, last_refill_time) = unpack_state(current_state);

            // Calculate refill (integer math only)
            let time_diff = now.saturating_sub(last_refill_time);
            let tokens_to_add = time_diff * self.tokens_per_second;
            // Hard limit: never allow more tokens than tokens_per_second (no burst)
            let available_tokens = (current_tokens + tokens_to_add).min(self.tokens_per_second);

            if available_tokens == 0 {
                return Err(RateLimitExceeded);
            }

            // Atomically: refill + consume 1 token + update timestamp
            let new_tokens = available_tokens - 1;
            let new_state = pack_state(new_tokens, now);

            if self.state.compare_exchange_weak(
                current_state, new_state,
                Ordering::Release, Ordering::Relaxed
            ).is_ok() {
                return Ok(());
            }
            // CAS failed, retry with updated values
        }
    }
}
```

2. **TargetServer Enhancement**
   - Add `rate_limiter: Option<Arc<RateLimiter>>` field
   - Initialize based on configuration

#### Thread-Safe Implementation
- **Lock-free design**: Single atomic operation combines refill + token consumption
- **Zero mutex contention**: No locks in the critical path
- **Compare-and-swap loop**: Handles concurrent access without blocking
- **Packed state**: 64-bit atomic stores both token count and timestamp
- **Hard limit enforcement**: tokens never exceed tokens_per_second
- **Performance**: True O(1) per request, scales with high concurrency

#### Race Condition Prevention
The key insight is that `get_best_target_servers()` returns multiple candidates (up to 4), but only one is actually used. To prevent race conditions:
- **DO NOT** consume tokens during server selection
- **DO** consume tokens atomically just before sending the actual request
- Multiple threads may select the same server, but only threads that successfully acquire tokens will proceed

### 3. Server Selection Integration

**Critical Design Decision**: Rate limit tokens must be consumed at the point of actual use, not during server selection, to avoid race conditions.

#### Two-Phase Approach:
1. **Selection Phase** (`get_best_target_servers()`):
   - Continue returning multiple servers as candidates
   - Do NOT consume rate limit tokens here
   - **Never block** - return empty list if all viable servers are at rate limit
   - All blocking/waiting logic is handled in execution phase


2. **Execution Phase** (in `proxy_server.rs` request loop):
   - Try to acquire a rate limit token immediately before sending request
   - If token not available, mark for potential retry but try next server first
   - Only the server actually used consumes a token
   - Important: Rate limit skip is different from failure - server remains eligible for retry

```rust
// In proxy_server.rs request loop
let mut rate_limited_servers = Vec::new();
for (server_idx, target_uri) in targets.iter() {
    // Non-blocking attempt to acquire token
    if let Some(target_server) = get_target_server(server_idx) {
        match target_server.try_acquire_rate_limit_token_nonblocking() {
            Ok(_) => {
                // Token acquired, proceed with request
                let resp = req_builder.send().await;
                // ... handle response (success or failure ends loop)
            }
            Err(RateLimitExceeded) => {
                // Rate limited - remember for potential retry
                rate_limited_servers.push(server_idx);
                stats.increment_rate_limited_skip();
                continue; // Try next server
            }
        }
    }
}

// If all servers were rate limited, wait for any to become available
if !rate_limited_servers.is_empty() && request_not_yet_handled {
    // Wait up to 3 seconds with brief random backoff
    let timeout = Duration::from_secs(3);
    let backoff = Duration::from_millis(rand::thread_rng().gen_range(10..100));
    thread::sleep(backoff);

    // Try to acquire token from any rate-limited server
    if let Some(server_idx) = wait_for_any_token(&rate_limited_servers, timeout) {
        // Retry with the server that now has tokens
        // ... send request to this server
    }
}
```


### 4. Statistics Tracking

#### Per-Server Statistics (ServerStats):
- `rate_limit_skips: u64` - Times server was skipped due to rate limit
- `rate_limit_blocks: u64` - Times request blocked waiting for tokens
- `rate_limit_percentage()` - Calculate percentage: `(rate_limit_skips + rate_limit_blocks) / total_requests`

#### Global Statistics (InputPort):
- `queue_time_sum: f64` - Sum of all queue times in milliseconds
- `queue_time_count: u64` - Number of queue time measurements
- `avg_queue_time_ms()` - Average queue time: `queue_time_sum / queue_time_count`

Queue time measures from request handler start to:
- Successful send initiation, or
- Final failure confirmation (after all retries)

This includes all server selection, retry logic, and rate limit blocking time.

### Implementation Notes (From Design Review)

#### Burst vs No-Burst Clarification
- **Hard Limit Enforced**: `available_tokens` is capped at `tokens_per_second`
- **No Burst Accumulation**: Even during idle periods, tokens never exceed the per-second limit
- **Immediate Enforcement**: Rate limit is strict from daemon startup

#### Precision and Performance
- **Integer-Only Math**: Use `u32` for `tokens_per_second` to avoid floating-point precision errors
- **Second-Level Granularity**: Timestamp stored as seconds since startup (sufficient for rate limiting)
- **Saturating Arithmetic**: Prevent integer overflow in calculations

### 5. API Enhancements

Update API responses to include rate limit statistics:
- Add rate limit info to server status
- Show skip percentage in monitoring APIs
- Include configuration in status reports

## Implementation Steps

### Phase 1: Core Rate Limiter COMPLETED
1. Create `rate_limiter.rs` with token bucket implementation (hard limit, no burst)
2. Add unit tests for concurrent access patterns
3. Benchmark performance with high concurrency. When testing, use a large mix of >100 highly available websites (e.g. google.com, cnn.com) with artificial very low rate limits.

### Phase 2: Configuration COMPLETED
1. Update Link struct in `workdirs.rs` to add `max_per_secs: Option<u32>` field
2. Parse max_per_secs from YAML
3. Add validation for max_per_secs (must be > 0 if specified)

### Phase 2B: Dual Rate Limiting (QPS + QPM) COMPLETED

Some RPC vendors use QPM (queries per minute) instead of QPS (queries per second). This phase adds support for configuring both rate limits simultaneously, where both must be satisfied for a request to proceed.

**Key Innovation**: Instead of using two separate rate limiters (which could lead to token loss under contention), this design uses a single unified atomic operation that handles both limits simultaneously, eliminating race conditions entirely.

#### Requirements
- Support zero, one, or both rate limiting rules
- Efficient algorithm that only checks configured limiters
- Both limits must be satisfied simultaneously
- Maintain lock-free performance characteristics

#### Configuration Schema Extension
Extend the Link configuration to include `max_per_min`:

```yaml
links:
  - alias: "sui.io"
    rpc: "https://fullnode.testnet.sui.io:443"
    ws: "wss://fullnode.testnet.sui.io:443"
    priority: 10
    max_per_secs: 100   # Optional: maximum requests per second
    max_per_min: 5000   # Optional: maximum requests per minute
```

**Configuration Examples:**
- QPS only: `max_per_secs: 50, max_per_min: null`
- QPM only: `max_per_secs: null, max_per_min: 2400`
- Both limits: `max_per_secs: 50, max_per_min: 2400`
- No limits: `max_per_secs: null, max_per_min: null`

#### Refactored RateLimiter Architecture

We refactor the existing `RateLimiter` to handle both QPS and QPM constraints simultaneously within one atomic operation. This clean design eliminates token loss entirely.

```rust
pub struct RateLimiter {
    max_per_secs: u32,    // 0 = unlimited, max 32,767 (15 bits)
    max_per_min: u32,     // 0 = unlimited, max 262,143 (18 bits)
    // Single atomic storing packed state (64 bits):
    // [minute_window_id: 15][minute_tokens: 18][second_window_id: 16][second_tokens: 15]
    state: AtomicU64,
    startup_time: Instant,
}

impl RateLimiter {
    /// Create a rate limiter with both QPS and QPM limits
    /// Use 0 for unlimited on either parameter
    /// Returns error if limits exceed bit field capacity
    pub fn new(max_per_secs: u32, max_per_min: u32) -> Result<Self, &'static str> {
        // Validate limits fit in their bit fields
        if max_per_secs > 32_767 {  // 15 bits
            return Err("max_per_secs exceeds 32,767 limit");
        }
        if max_per_min > 262_143 {  // 18 bits
            return Err("max_per_min exceeds 262,143 limit");
        }

        Ok(Self {
            max_per_secs,
            max_per_min,
            state: AtomicU64::new(0),
            startup_time: Instant::now(),
        })
    }

    pub fn try_acquire_token(&self) -> Result<(), RateLimitExceeded> {
        // Single atomic operation handles both limits simultaneously
    }

    /// Check available tokens for monitoring (returns min of both limits)
    pub fn tokens_available(&self) -> u32 {
        let current_time_secs = self.get_current_time_seconds();
        let current_second_id = current_time_secs & 0xFFFF;
        let current_minute_id = (current_time_secs / 60) & 0x7FFF;

        let current_state = self.state.load(Ordering::Acquire);
        let (last_min_id, last_min_tokens, last_sec_id, last_sec_tokens) = unpack_state(current_state);

        // Calculate current token availability (non-consuming)
        let available_sec_tokens = if self.max_per_secs == 0 {
            u32::MAX // Unlimited
        } else if self.has_window_expired(current_second_id, last_sec_id, true) {
            self.max_per_secs
        } else {
            last_sec_tokens
        };

        let available_min_tokens = if self.max_per_min == 0 {
            u32::MAX // Unlimited
        } else if self.has_window_expired(current_minute_id, last_min_id, false) {
            self.max_per_min
        } else {
            last_min_tokens
        };

        // Return the limiting factor
        available_sec_tokens.min(available_min_tokens)
    }
}
```

#### Unified Atomic Algorithm: True All-or-Nothing

The algorithm uses a single `compare_exchange_weak` loop that handles refill, availability checking, and token consumption for both time windows in one atomic operation.

**Critical Race Condition Prevention:**
The algorithm calculates intended token counts independently of the loaded state to prevent the "stale refill" race condition. Without this fix, multiple threads competing during window transitions could lose refill opportunities, reintroducing token loss under high contention.

**Unlimited Value Representation:**
When a limit is disabled (set to 0), its corresponding token count in the packed state is set to the maximum possible value (`0x7FFF` for seconds, `0x3FFFF` for minutes) to signify "unlimited" capacity.

**Fair Contention Handling:**
The algorithm distinguishes between temporary unavailability (window transition races) and true rate limiting. It continues looping (via `continue`) when tokens are unavailable, anticipating an upcoming window refresh. It only returns `Err(RateLimitExceeded)` if a window has already been refreshed (`seen_fresh_window` is true) and the token count is still zero, indicating a true rate-limited state. This prevents both temporary starvation and infinite loops.

**Tokio-Friendly Design:**
To prevent hot spinning in async contexts, the algorithm:
- Recalculates time on each iteration to detect window changes
- Uses `std::thread::yield_now()` every 10 iterations when waiting for tokens
- Enforces a 2-second timeout to prevent indefinite blocking of tokio runtime threads

**Packed State Format (64 bits):**
```
| minute_window_id (15) | minute_tokens (18) | second_window_id (16) | second_tokens (15) |
```

**Algorithm Implementation:**
```rust
pub fn try_acquire_token(&self) -> Result<(), RateLimitExceeded> {
    let start_time_secs = self.get_current_time_seconds();
    const TIMEOUT_SECS: u32 = 2; // Prevent indefinite blocking in async context

    // Track if we've seen a fresh window but still no tokens (true rate limit)
    let mut seen_fresh_window = false;
    let mut iterations = 0u32;

    loop {
        // Recalculate time inside loop to detect window changes
        let current_time_secs = self.get_current_time_seconds();
        let current_second_id = current_time_secs & 0xFFFF;  // 16 bits
        let current_minute_id = (current_time_secs / 60) & 0x7FFF;  // 15 bits

        // Timeout protection for tokio context
        if current_time_secs > start_time_secs + TIMEOUT_SECS {
            return Err(RateLimitExceeded);
        }
        let current_state = self.state.load(Ordering::Acquire);
        let (last_min_id, last_min_tokens, last_sec_id, last_sec_tokens) = unpack_state(current_state);

        // Calculate intended token counts based on current time (prevents stale refill race)
        let sec_window_fresh = self.has_window_expired(current_second_id, last_sec_id, true);
        let min_window_fresh = self.has_window_expired(current_minute_id, last_min_id, false);

        let mut available_sec_tokens = if self.max_per_secs == 0 {
            0x7FFF // Unlimited: use max possible value
        } else if sec_window_fresh {
            seen_fresh_window = true;
            self.max_per_secs // Window expired: full refill
        } else {
            last_sec_tokens // Same window: use current count
        };

        let mut available_min_tokens = if self.max_per_min == 0 {
            0x3FFFF // Unlimited: use max possible value
        } else if min_window_fresh {
            seen_fresh_window = true;
            self.max_per_min // Window expired: full refill
        } else {
            last_min_tokens // Same window: use current count
        };

        // Check availability and consume tokens atomically (prevents premature failure)
        if self.max_per_secs > 0 {
            if available_sec_tokens == 0 {
                // No tokens available. If we've seen a fresh window but still no tokens,
                // this is a true rate limit. Otherwise, continue waiting for next window.
                if seen_fresh_window {
                    return Err(RateLimitExceeded);
                }
                // Yield CPU to prevent hot spinning in tokio context
                iterations += 1;
                if iterations % 10 == 0 {  // Yield every 10 iterations
                    std::thread::yield_now();
                }
                continue;
            }
            available_sec_tokens -= 1;
        }

        if self.max_per_min > 0 {
            if available_min_tokens == 0 {
                // No tokens available. If we've seen a fresh window but still no tokens,
                // this is a true rate limit. Otherwise, continue waiting for next window.
                if seen_fresh_window {
                    return Err(RateLimitExceeded);
                }
                // Yield CPU to prevent hot spinning in tokio context
                iterations += 1;
                if iterations % 10 == 0 {  // Yield every 10 iterations
                    std::thread::yield_now();
                }
                continue;
            }
            available_min_tokens -= 1;
        }

        // Attempt atomic update with calculated tokens
        let new_state = pack_state(current_minute_id, available_min_tokens, current_second_id, available_sec_tokens);

        if self.state.compare_exchange_weak(
            current_state, new_state,
            Ordering::Release, Ordering::Relaxed
        ).is_ok() {
            return Ok(()); // Success: both limits satisfied atomically
        }
        // CAS failed, retry with fresh state
    }
}

/// Get current time in seconds since daemon startup
fn get_current_time_seconds(&self) -> u32 {
    self.startup_time.elapsed().as_secs() as u32
}

// Determines if a new time window has started, correctly handling both normal time progression and timer wrap-around
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
```

#### Advantages of Unified Approach

**Eliminates Token Loss:**
- Both limits are checked and decremented in a single atomic operation
- Impossible to consume tokens from one bucket without the other
- No race conditions between separate limit checks

**Better Under Contention:**
- Accuracy improves when it matters most (high load scenarios)
- No systematic bias toward depleting one bucket over another
- Maintains precise enforcement of both configured limits

**Performance Characteristics:**
- Single atomic operation instead of two separate ones
- Potentially faster due to reduced memory bandwidth
- Still completely lock-free with same scalability properties

#### Bit Packing Details

The 64-bit state efficiently stores all rate limiting information with optimized allocation:

```rust
// Pack: 15-bit minute ID, 18-bit minute tokens, 16-bit second ID, 15-bit second tokens
fn pack_state(min_id: u32, min_tokens: u32, sec_id: u32, sec_tokens: u32) -> u64 {
    ((min_id as u64 & 0x7FFF) << 49) |      // 15 bits: minute window ID
    ((min_tokens as u64 & 0x3FFFF) << 31) | // 18 bits: minute tokens
    ((sec_id as u64 & 0xFFFF) << 15) |      // 16 bits: second window ID
    (sec_tokens as u64 & 0x7FFF)            // 15 bits: second tokens
}

// Unpack 64-bit state into four values with correct bit masks
fn unpack_state(state: u64) -> (u32, u32, u32, u32) {
    let min_id = ((state >> 49) & 0x7FFF) as u32;        // 15 bits
    let min_tokens = ((state >> 31) & 0x3FFFF) as u32;   // 18 bits
    let sec_id = ((state >> 15) & 0xFFFF) as u32;        // 16 bits
    let sec_tokens = (state & 0x7FFF) as u32;            // 15 bits
    (min_id, min_tokens, sec_id, sec_tokens)
}
```

**Optimized Capacity Limits:**
- **Second tokens**: 15 bits = 32,767 max QPS (excellent for any realistic workload)
- **Minute tokens**: 18 bits = 262,143 max QPM (4x increase, handles enterprise limits)
- **Window IDs**:
  - Second windows: 16 bits = 65,535 windows (~18 hours before wrap-around)
  - Minute windows: 15 bits = 32,767 windows (~22 days before wrap-around)
- **Wrap-around handling**: Robust detection ensures year+ uptime support

#### Performance Characteristics

**Memory Overhead**:
```rust
// Current RateLimiter (single limit)
pub struct RateLimiter {
    tokens_per_second: u32,     // 4 bytes
    state: AtomicU64,           // 8 bytes
    startup_time: Instant,      // 8 bytes
}
// Total: ~24 bytes + padding

// Refactored RateLimiter (dual limit)
pub struct RateLimiter {
    max_per_secs: u32,          // 4 bytes
    max_per_min: u32,           // 4 bytes (new)
    state: AtomicU64,           // 8 bytes (packed with more data)
    startup_time: Instant,      // 8 bytes
}
// Total: ~32 bytes + padding
```

- **8 byte overhead** per server for dual rate limiting capability
- Single 8-byte atomic operation regardless of configuration
- No additional data structures or coordination overhead

**CPU Overhead**:
- Single atomic load per request attempt
- Single atomic CAS operation for token consumption
- No separate availability checks or token coordination
- Bit manipulation operations are extremely fast (sub-nanosecond)

**Lock-free Guarantee**:
- One atomic operation handles entire rate limiting decision
- No coordination between separate limiters
- Scales linearly with thread count under contention

#### Implementation Steps

1. **Extend Link Configuration**
   - Add `max_per_min: Option<u32>` to Link struct in `common/src/shared_types/workdirs.rs`
   - Update YAML parsing and validation for the new field
   - Validate max_per_secs ≤ 32,767 and max_per_min ≤ 262,143 during parsing

2. **Refactor RateLimiter Implementation**
   - Replace existing `RateLimiter` struct in `rate_limiter.rs` with dual-limit version
   - Change constructor signature to `new(max_per_secs: u32, max_per_min: u32)`
   - Implement bit packing/unpacking functions for 64-bit state
   - Replace current algorithm with unified `try_acquire_token()` implementation
   - Update `tokens_available()` method to handle dual limits

3. **Update All Usage Sites**
   - Update existing `RateLimiter::new(tokens_per_second)` calls to `new(tokens_per_second, 0)`
   - Update TargetServer initialization to pass both rate limits from configuration
   - Update all tests to use new constructor signature

4. **Comprehensive Testing**
   - Unit tests for all rate limit combinations (QPS only, QPM only, both, neither)
   - Bit packing/unpacking correctness tests
   - Concurrent access tests with dual limiting
   - Performance benchmarks comparing old vs. new implementation
   - Edge case testing (window boundary conditions, overflow protection)

#### Integration with Phase 3

Phase 3 will update `TargetServer` to pass both rate limits from Link configuration to the refactored `RateLimiter::new(max_per_secs, max_per_min)` constructor.

#### Future Considerations

For better tokio integration, consider making the rate limiter async-aware in a future phase:
- Use `tokio::time::sleep` instead of `thread::yield_now()` for waiting
- Make `try_acquire_token` an async function
- This would prevent blocking tokio runtime threads entirely
- However, this requires more significant architectural changes

### Phase 3: Integration NEXT TO BE DONE (after Phase 2B)

1. Update TargetServer to use refactored RateLimiter with dual-limit support
2. Modify server selection logic in `get_best_target_servers()`
3. Implement blocking behavior when all servers rate limited

### Phase 4: Statistics
1. Extend ServerStats with rate limit metrics (skips, blocks, percentage)
2. Add queue time tracking to InputPort (global metric)
3. Update ProxyServer to measure queue time from handler start
4. Update NetworkMonitor to track rate limit events
5. Add rate limit and queue time info to API responses

### Phase 5: Testing
1. Unit tests for rate limiter
2. Integration tests for server selection with rate limits
3. Performance tests under high load
4. Test rate limit statistics accuracy

## Performance Considerations

1. **Lock-Free Token Bucket**: O(1) for checking/consuming tokens with zero mutex contention
2. **Single Atomic Operation**: Refill + consumption in one compare-and-swap
3. **High Concurrency**: Scales well under load, no blocking on rate limit checks
4. **Packed State**: Efficient use of cache lines with 64-bit atomic state

## Edge Cases

1. **Configuration Changes**: Hot-reload rate limits via WorkdirsWatcher
2. **Clock Drift**: Use monotonic clocks (Instant) for timing
3. **Integer Overflow**: Use saturating arithmetic for token calculations
4. **Startup Behavior**: Initialize with zero tokens (enforce limit from start)
5. **All Servers Rate Limited**: Implement fair waiting queue
6. **Hard Limit**: No burst allowance - tokens capped at tokens_per_second
7. **Integer Arithmetic**: Avoid floating-point precision issues with integer-only math
8. **RequestBuilder Cloning**: Handle reqwest consumption with proper cloning for retries