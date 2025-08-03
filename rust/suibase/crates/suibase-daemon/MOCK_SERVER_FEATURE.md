# Mock Server Testing Framework

## Overview

Complete testing framework for suibase-daemon proxy server using mock servers to simulate RPC server behaviors, failures, and rate limits. Implementation complete and production-ready.

## Architecture

### Mock Server Detection
- Mock servers identified by `"mock-"` prefix in alias (e.g., `mock-0`, `mock-1`)
- Run as async HTTP tasks within suibase-daemon process (NOT separate processes)
- Managed by `MockServerManager` subsystem following standard patterns
- **IMPORTANT**: Mock servers are only created for the `localnet` workdir

### Key Components

**Files:**
- `src/mock_server_manager.rs` - Subsystem managing mock server lifecycle
- `src/workers/mock_server.rs` - HTTP server implementation with behavior simulation
- `src/shared_types/mock_server.rs` - Types and state management
- `src/api/impl_mock_api.rs` - JSON-RPC API for mock control
- `tests/*_tests.rs` - Reorganized test suites (see Testing section)
- `tests/common/mock_test_utils.rs` - Test utilities and harness

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
Get statistics for mock server (read-only):
```json
{
  "method": "mockServerStats",
  "params": {"alias": "mock-0"}
}
```

### mockServerReset
Reset statistics for mock server:
```json
{
  "method": "mockServerReset",
  "params": {"alias": "mock-0"}
}
```

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

### Security & Restrictions
- Mock servers ONLY run in `localnet` workdir (hardcoded check)
- Alias MUST start with `"mock-"` prefix
- API methods validate alias format before processing
- No bypass mechanism exists for these restrictions

## Testing

### Test Organization (NEW)
Tests have been reorganized into three focused test suites:

**1. mock_server_api_tests.rs**
- Tests mock server API functionality
- Behavior configuration, statistics, caching
- Direct mock server features

**2. rate_limiting_tests.rs**
- Tests rate limiting enforcement
- QPS/QPM dual limits
- Dynamic configuration changes
- Load balancing with rate limits

**3. proxy_behavior_tests.rs**
- Tests proxy server routing logic
- Load balancing, failover, retry
- Server selection behavior
- Mixed server scenarios

### Test Harness Pattern
The `MockServerTestHarness` provides:
- Daemon lifecycle management
- Configuration backup/restore
- RAII cleanup even on panic
- Helper methods for common test operations

### Running Tests
```bash
# Recommended: Use the test script for safe execution
~/suibase/scripts/dev/test-mock-servers

# Or run individual test suites
cargo test --test mock_server_api_tests -- --test-threads=1
cargo test --test rate_limiting_tests -- --test-threads=1
cargo test --test proxy_behavior_tests -- --test-threads=1
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
2. Workdir check ensures only `localnet` is processed
3. Started as async tasks by `MockServerManager`
4. Inherit rate limiters from Link configuration
5. Controlled via messaging through `AdminController`

### Rate Limiting Integration
Mock servers automatically inherit rate limiting from their Link configuration:
- Rate limiter created during `MockServerState` initialization
- Configuration updates trigger rate limiter recreation
- Mock servers check rate limits before processing requests
- See `update_rate_limiter()` in `MockServerState`

### Debugging
- Extensive logging in MockServerManager and workers
- Statistics available via existing `getLinks` API
- Mock-specific stats via `mockServerStats` API
- Test harness provides diagnostic helpers

## Production Considerations

1. **Performance:** Zero impact when mock servers not configured
2. **Safety:** Mock servers only run with explicit `"mock-"` alias prefix AND localnet workdir
3. **Isolation:** Each mock server runs in separate async task
4. **Cleanup:** Proper shutdown via `tokio-graceful-shutdown`
5. **Thread Safety:** All shared state uses `Arc<RwLock<>>`

## Common Issues

### Mock Server Not Receiving Traffic
- Check `selectable: true` in configuration
- Verify proxy server selection logic via `getLinks` API
- Ensure mock server is healthy (responding to requests)
- Confirm workdir is `localnet`

### Configuration Not Applied
- Use config timestamp mechanism for reliable change detection
- Check `WorkdirsWatcher` is processing file changes
- Verify `AdminController` is handling `ConfigUpdate` messages

### Test Failures
- Ensure daemon restart between incompatible config changes
- Run tests serially with `--test-threads=1`
- Check for proper cleanup in test harness
- Verify no leftover mock server configurations

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

Mock server events use dedicated constants for message passing:
- `EVENT_MOCK_SERVER_CONFIG`: Configuration updates
- `EVENT_MOCK_SERVER_CONTROL`: Behavior control messages
- `EVENT_MOCK_SERVER_STATS_RESET`: Statistics reset requests

## Future Maintenance

This feature is designed to be stable and low-maintenance. Key invariants to preserve:
1. Mock servers only run in `localnet` workdir
2. Mock servers only run with `"mock-"` prefix
3. All mutable operations go through messaging
4. `selectable: false` servers never receive traffic
5. Rate limiting inheritance from Link config
6. Graceful shutdown handling