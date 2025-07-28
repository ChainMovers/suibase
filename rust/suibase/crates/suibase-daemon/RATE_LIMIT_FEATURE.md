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
        let now = get_monotonic_time_u32(); // Seconds since startup

        loop {
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

### Phase 3: Integration NEXT TO BE DONE
1. Add RateLimiter to TargetServer
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