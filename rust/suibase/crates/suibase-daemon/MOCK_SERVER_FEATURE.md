# Mock Server Testing Framework

## Overview

Complete testing framework for suibase-daemon proxy server using mock servers to simulate RPC server behaviors, failures, and rate limits. Implementation complete and production-ready.

## Architecture

### Mock Server Detection
- Mock servers identified by `"mock-"` prefix in alias (e.g., `mock-0`, `mock-1`)
- Run as async HTTP tasks within suibase-daemon process (NOT separate processes)
- Managed by `MockServerManager` subsystem following standard patterns

### Key Components

**Files:**
- `src/mock_server_manager.rs` - Subsystem managing mock server lifecycle
- `src/workers/mock_server.rs` - HTTP server implementation with behavior simulation
- `src/shared_types/mock_server.rs` - Types and state management
- `src/api/impl_mock_api.rs` - JSON-RPC API for mock control
- `tests/mock_server_integration_tests.rs` - Integration tests
- `tests/common/mock_test_utils.rs` - Test utilities

**Configuration Example (suibase.yaml):**
```yaml
links:
  - alias: "localnet"
    rpc: "http://localhost:9000"
    selectable: false  # CRITICAL during testing
    monitored: true

  - alias: "mock-0"
    rpc: "http://localhost:50001"
    selectable: true
    monitored: true
    max_per_secs: 10  # Rate limiting inherited by mock server
```

## JSON-RPC API

### mockServerControl
Control individual mock server behavior:
```json
{
  "method": "mockServerControl",
  "params": {
    "alias": "mock-0",
    "action": "set_behavior",
    "behavior": {
      "failure_rate": 0.5,
      "latency_ms": 2000,
      "http_status": 200,
      "response_body": {
        "jsonrpc": "2.0", "id": 1,
        "error": {"code": -32000, "message": "object notExists"}
      }
    }
  }
}
```

**Actions:** `set_behavior`, `reset`, `pause`, `resume`

### mockServerStats
Get statistics for mock server:
```json
{
  "method": "mockServerStats",
  "params": {"alias": "mock-0", "reset_after": true}
}
```

### mockServerBatch
Control multiple servers at once.

## Implementation Details

### Subsystem Architecture
- `MockServerManager` runs as proper subsystem (like netmon, acoinsmon)
- Uses message-passing for all mutable operations via `AdminController`
- No direct API access to manager state
- Follows run/event_loop pattern with graceful shutdown

### Configuration Flow
1. `WorkdirsWatcher` detects suibase.yaml changes
2. `AdminController` processes config via messaging
3. Mock servers inherit rate limits from Link configuration
4. Hot-reload support for behavior changes (no restart needed)

## Testing

### Test Harness Pattern
```rust
// tests/common/mock_test_utils.rs
pub struct MockServerTestHarness {
    // Handles daemon lifecycle, config backup/restore
    // RAII cleanup even on panic
}
```

### Key Tests
- `test_selectable_flag_respected` - Verifies real servers get 0% traffic
- `test_load_balancing` - Load distribution across healthy servers
- `test_failover_behavior` - Server failure handling
- `test_proxy_server_rate_limiting` - Rate limiting enforcement
- `test_cascading_failures` - Graceful degradation

### Running Tests
```bash
# Recommended: Use the test script for safe execution
~/suibase/scripts/dev/test-mock-servers

# Or run directly (must be sequential)
cargo test --test mock_server_integration_tests -- --test-threads=1
```

**Important:** Tests run serially to prevent daemon conflicts.

## Maintenance Notes

### Configuration Changes
**Hot-reload (no restart):**
- `selectable`, `max_per_secs`, `max_per_min`, `monitored` flags
- Mock server behavior via API

**Requires restart:**
- Adding/removing mock servers
- Changing ports

### Mock Server Lifecycle
1. Detected by `"mock-"` prefix during config parsing
2. Started as async tasks by `MockServerManager`
3. Inherit rate limiters from Link configuration
4. Controlled via messaging through `AdminController`

### Rate Limiting Integration
Mock servers inherit `RateLimiter` from their Link configuration:
```rust
// In MockServerState
pub rate_limiter: Arc<RwLock<Option<RateLimiter>>>,

// Inheritance on config update
pub fn update_rate_limiter(&self, link_config: &Link) {
    let new_rate_limiter = Self::create_rate_limiter_from_config(link_config);
    // ...
}
```

### Debugging
- Extensive logging in MockServerManager and workers
- Statistics available via existing `getLinks` API
- Mock-specific stats via `mockServerStats` API

## Production Considerations

1. **Performance:** Zero impact when mock servers not configured
2. **Safety:** Mock servers only run with explicit `"mock-"` alias prefix
3. **Isolation:** Each mock server runs in separate async task
4. **Cleanup:** Proper shutdown via `tokio-graceful-shutdown`
5. **Thread Safety:** All shared state uses `Arc<RwLock<>>`

## Common Issues

### Mock Server Not Receiving Traffic
- Check `selectable: true` in configuration
- Verify proxy server selection logic via `getLinks` API
- Ensure mock server is healthy (responding to requests)

### Configuration Not Applied
- Use config timestamp mechanism for reliable change detection
- Check `WorkdirsWatcher` is processing file changes
- Verify `AdminController` is handling `ConfigUpdate` messages

### Test Failures
- Ensure daemon restart between incompatible config changes
- Run tests serially with `--test-threads=1`
- Check for proper cleanup in test harness

### Development Scripts

**test-mock-servers**
```bash
~/suibase/scripts/dev/test-mock-servers
```
- Runs integration tests safely with sequential execution
- Stops on first failure for easier debugging
- Prevents race conditions over shared daemon/config
- Includes helpful debugging tips

**restore-default-config** 
```bash
~/suibase/scripts/dev/restore-default-config
```
- Restores default suibase.yaml from `~/suibase/scripts/defaults/localnet/`
- Useful for cleanup after debugging test failures
- Daemon auto-reloads when config changes

## Event Constants

```rust
// common/basic_types.rs
pub const EVENT_MOCK_SERVER_CONFIG: u8 = 132;
pub const EVENT_MOCK_SERVER_CONTROL: u8 = 133;
pub const EVENT_MOCK_SERVER_STATS_RESET: u8 = 134;
pub const EVENT_MOCK_SERVER_BATCH_CONTROL: u8 = 135;
```

## Future Maintenance

This feature is designed to be stable and low-maintenance. Key invariants to preserve:
1. Mock servers only run with `"mock-"` prefix
2. All mutable operations go through messaging
3. `selectable: false` servers never receive traffic
4. Rate limiting inheritance from Link config
5. Graceful shutdown handling
