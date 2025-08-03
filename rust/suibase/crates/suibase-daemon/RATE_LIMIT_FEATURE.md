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
- Links defined in `suibase.yaml` with fields: alias, rpc, ws, priority, selectable, monitored, max_per_secs, max_per_min
- Each workdir has its own configuration

## Implementation

### 1. Configuration Schema
Add rate limiting fields to the Link configuration in `suibase.yaml`:

```yaml
links:
  - alias: "sui.io"
    rpc: "https://fullnode.testnet.sui.io:443"
    ws: "wss://fullnode.testnet.sui.io:443"
    priority: 10
    max_per_secs: 100  # Optional: maximum requests per second
    max_per_min: 5000  # Optional: maximum requests per minute
```

### 2. Rate Limiter Architecture

#### Core Components

**RateLimiter struct** (`rate_limiter.rs`)
- Unified dual-limit token bucket algorithm (QPS + QPM)
- Lock-free design using single 64-bit atomic operation
- Handles both per-second and per-minute limits simultaneously
- Zero token loss under high contention

**Key Algorithm Features:**
- **Packed State Format**: Efficiently stores window IDs and token counts in 64 bits
- **Window-based Tracking**: Independent second and minute windows with automatic refill
- **Wraparound Handling**: Robust handling of timer wraparound for year+ uptime
- **Fair Contention**: Distinguishes between temporary unavailability and true rate limiting
- **Tokio-Friendly**: Yields CPU to prevent hot spinning in async contexts

**Capacity Limits:**
- Second tokens: 15 bits = 32,767 max QPS
- Minute tokens: 18 bits = 262,143 max QPM
- Window IDs support year+ daemon uptime

#### Thread-Safe Implementation
- **Lock-free design**: Single atomic operation combines refill + token consumption
- **Zero mutex contention**: No locks in the critical path
- **Compare-and-swap loop**: Handles concurrent access without blocking
- **Packed state**: 64-bit atomic stores both token counts and timestamps
- **Hard limit enforcement**: Tokens never exceed configured limits
- **Performance**: True O(1) per request, scales with high concurrency

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

### 4. Statistics Tracking

#### Per-Server Rate Limiting Statistics

Each RPC server tracks and displays three key metrics:

1. **QPS (Queries Per Second)** - Current queries per second from actual token consumption
2. **QPM (Queries Per Minute)** - Current queries per minute from actual token consumption
3. **LIMIT** - Cumulative count of rate limit occurrences

#### Display Format

The status display shows rate limiting statistics in a compact format using 7-character wide columns:

```
alias                Status  Health%   Load%   RespT ms  Success%    QPS    QPM   LIMIT
------------------------------------------------------------------------------------------
mock-2                 OK  * +100.0    16.2       0.87     100.0     15    900       0
mock-0                 OK  * +100.0    29.3       0.88     100.0     28   1680       0
mock-4                 OK  * +100.0    20.0       0.88     100.0     19   1140      2K
mock-3                 OK  * +100.0    13.3       0.91     100.0     12    720     15K
mock-1                 OK  * +100.0    21.2       1.06     100.0     21   1260    125K
localnet               OK    +100.0     0.0       1.02         -      0      0       0

* is the load-balanced range on first attempt.
On retry, another OK server is selected in order shown in table.
```

**Formatting Rules:**
- Values 0-999: Show as integer (e.g., "    123")
- Values 1000-999999: Show with K suffix (e.g., "    45K")
- Values ≥1000000: Show with M suffix (e.g., "     2M")
- No decimal points - keep values as integers

### 5. API Enhancements

Update API responses to include rate limit statistics:

**LinkStats Structure Enhancement:**
- Formatted fields for display mode: `qps`, `qpm`, `rate_limit_count`
- Raw numeric fields for data mode: `qps_raw`, `qpm_raw`, `rate_limit_count_raw`
- Allows API consumers to format data as needed

## Implementation Status

### Phase 1: Core Rate Limiter ✅ COMPLETED
- Created `rate_limiter.rs` with token bucket implementation
- Unit tests for concurrent access patterns
- Performance benchmarks with high concurrency

### Phase 2: Configuration ✅ COMPLETED
- Updated Link struct to add `max_per_secs` field
- Parse max_per_secs from YAML
- Validation for max_per_secs

### Phase 2B: Dual Rate Limiting (QPS + QPM) ✅ COMPLETED
- Support for both per-second and per-minute limits
- Unified atomic algorithm prevents token loss
- Backwards compatible with existing configurations

### Phase 3: Integration ✅ COMPLETED
- TargetServer uses dual-limit RateLimiter
- Server selection respects rate limits
- Request handler enforces limits before sending
- Hot configuration reload support

### Phase 4: Statistics ✅ COMPLETED
- QPS/QPM calculation from rate limiter
- Cumulative LIMIT tracking in ServerStats
- Display formatting with K/M units
- API enhancements for rate limit data

### Phase 5: Testing ✅ COMPLETED
- Comprehensive unit tests for rate limiter
- Integration tests for server selection
- Performance tests under high load
- Statistics accuracy validation

## Performance Considerations

1. **Lock-Free Token Bucket**: O(1) for checking/consuming tokens with zero mutex contention
2. **Single Atomic Operation**: Refill + consumption in one compare-and-swap
3. **High Concurrency**: Scales well under load, no blocking on rate limit checks
4. **Packed State**: Efficient use of cache lines with 64-bit atomic state

## Edge Cases

1. **Configuration Changes**: Hot-reload rate limits via WorkdirsWatcher
2. **Clock Drift**: Use monotonic clocks (Instant) for timing
3. **Integer Overflow**: Use saturating arithmetic for token calculations
4. **Startup Behavior**: Initialize with full tokens (immediate availability)
5. **All Servers Rate Limited**: Graceful fallback to include rate-limited servers
6. **Hard Limits**: No burst allowance - tokens capped at configured limits
7. **Integer Arithmetic**: Avoid floating-point precision issues
8. **Wraparound Handling**: Robust detection for year+ uptime support

## Mock Server Integration

Mock servers automatically inherit rate limiting from their Link configuration:
- Rate limiters created during MockServerState initialization
- Configuration updates trigger rate limiter recreation
- Mock servers can simulate rate limiting behavior for testing
- See `MOCK_SERVER_FEATURE.md` for details on mock server framework