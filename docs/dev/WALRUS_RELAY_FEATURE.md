# Walrus Upload Relay Integration

## Summary

Integrate Walrus upload relay via suibase-daemon proxy. Maintains 100% API compatibility while enabling future extensibility.

## Architecture

```
Application → suibase-daemon:45852 → walrus-upload-relay:45802  # testnet
Application → suibase-daemon:45853 → walrus-upload-relay:45803  # mainnet
```

**Proxy Ports (458XX range):**
- Testnet: 45852  (configurable)
- Mainnet: 45853  (configurable)

**Backend Relay Ports:**
- Testnet: 45802 (configurable)
- Mainnet: 45803 (configurable)

**Components:**
- suibase-daemon: Transparent proxy per network
- walrus-upload-relay: Separate binary per network with configurable ports

**Endpoints:** All HTTP requests forwarded transparently

## User Commands

Following autocoins pattern:
```bash
testnet wal-relay status    # Show current status
testnet wal-relay enable    # Enable relay proxy
testnet wal-relay disable   # Disable relay proxy
mainnet wal-relay status    # Same for mainnet
mainnet wal-relay enable
mainnet wal-relay disable
```

## Status Hierarchy

The status system uses a layered approach with CLI-detected states taking precedence over daemon-written states:

### Instantaneous CLI Detection (Highest Priority)
1. **DISABLED** - `walrus_relay_enabled: false` in suibase.yaml (takes precedence over all other states)
2. **STOPPED** - Workdir services are stopped (testnet stop)
3. **NOT RUNNING** - suibase-daemon is not running

### Daemon-Written Status (Lower Priority)
4. **OK** - Process running and health checks pass
5. **DOWN** - Process not running or health checks fail
6. **INITIALIZING** - Brief default state until daemon determines OK/DOWN

**Precedence Order**: DISABLED → STOPPED → NOT RUNNING → OK/DOWN/INITIALIZING

**Status File**: `workdirs/{network}/walrus-relay/status.yaml` (written by WalrusMonitor)


### Status Display Format
```
Walrus Relay     : OK ( pid 1223131 ) http://localhost:45852
Walrus Relay     : DOWN http://localhost:45852
Walrus Relay     : DISABLED
Walrus Relay     : STOPPED
Walrus Relay     : NOT RUNNING http://localhost:45852
Walrus Relay     : INITIALIZING ( pid 1223131 ) http://localhost:45852
```

**Notes:**
- Uses 17-character left-aligned label (same as "Proxy server")
- PID is the walrus-upload-relay backend process (not suibase-daemon)
- URL shows suibase-daemon proxy port (where users connect)
- DISABLED and STOPPED show no URL/PID (service unavailable)
- Only OK/DOWN states may have race conditions requiring test timing

## Implementation Status

**✅ Phase 1: COMPLETE** - Binary Process Management  
**✅ Phase 2: COMPLETE** - Bash Scripts and Command Integration  
**✅ Phase 3: COMPLETE** - Suibase-daemon Integration (status monitoring + simple request statistics)
**❌ Phase 4: TODO** - HTTP Proxy Implementation  

Current status: Configuration, status reporting, and request statistics tracking work. HTTP proxy forwarding not yet implemented.

## Implementation Phases

### Phase 1: Binary Process Management ✅ COMPLETE
**Process lifecycle setup:**
- Binary managed via existing walrus binary management system
- Create `scripts/common/__walrus-relay-process.sh` with:
  - `start_walrus_relay_process()` - Start walrus-upload-relay on configured port
  - `stop_walrus_relay_process()` - Graceful shutdown
  - `update_WALRUS_RELAY_PROCESS_PID_var()` - Track process PID
- Process command: `walrus-upload-relay --walrus-config walrus-config.yaml --server-address 0.0.0.0:45802 --relay-config relay-config.yaml`
- Health check options:
  - Primary: `curl http://localhost:45802/v1/tip-config` (returns JSON tip configuration)
  - Alternative: `curl http://localhost:45802/v1/api` (returns API specification)

**Integration points:**
- Binary location: `~/suibase/workdirs/{testnet,mainnet}/bin/walrus-upload-relay`
- Config location: `~/suibase/workdirs/{testnet,mainnet}/config-default/relay-config.yaml`
- Walrus client config: `~/suibase/workdirs/{testnet,mainnet}/config-default/walrus-config.yaml`
- Log location: `~/suibase/workdirs/{testnet,mainnet}/walrus-relay/walrus-relay-process.log`
- Hook into existing `testnet update` and `mainnet update` commands

### Phase 2: Bash Scripts and Command Integration ✅ COMPLETE
**Files to create/modify:**
- Functions in `scripts/common/__walrus-relay-process.sh`:
  - `walrus_relay_status()` - Use existing echo_process() with relay PID and proxy URL
  - `walrus_relay_enable()` - Set walrus_relay_enabled: true in suibase.yaml
  - `walrus_relay_disable()` - Set walrus_relay_enabled: false in suibase.yaml
  - `update_walrus_relay_status_yaml()` - Parse status.yaml from daemon
- Modify `scripts/common/__workdir-exec.sh`:
  - Add `walrus-relay) CMD_WALRUS_RELAY_REQ=true ;;` to command parsing
  - Add subcommand parsing for status|enable|disable
  - Add execution block calling walrus_relay_* functions

**Configuration in suibase.yaml:**
```yaml
walrus_relay_enabled: false
walrus_relay_proxy_port: 45852  # 45853 for mainnet
walrus_relay_local_port: 45802  # 45803 for mainnet
```

**Testing after Phase 2:**
- `testnet wal-relay enable` should update configuration
- `testnet wal-relay status` should show relay status and proxy URL
- Configuration commands work without daemon running

### Phase 3: Suibase-daemon Status Monitoring ✅ COMPLETE
**Rust files implemented:**
- `rust/suibase/crates/suibase-daemon/src/walrus_monitor.rs`:
  - Monitor walrus_relay_enabled configuration changes
  - Track walrus-upload-relay process health
  - Write status to `workdirs/{testnet,mainnet}/walrus-relay/status.yaml`
  - Simple request statistics tracking (total, success, failure counts)
  - API-accessible statistics (similar to JSON-RPC network stats)
- Integration with `rust/suibase/crates/suibase-daemon/src/admin_controller.rs`:
  - WalrusMonitor integration for status monitoring and stats collection

**Status management:**
- ✅ Daemon writes: `~/suibase/workdirs/{testnet,mainnet}/walrus-relay/status.yaml`
- ✅ Status values: DISABLED, INITIALIZING, OK, DOWN
- ✅ Include backend connectivity info

**Request statistics (Phase 3):**
- ✅ Simple WalrusStats structure: request/success/failure counts
- ✅ Event-based statistics collection via message passing
- ✅ API-accessible (not stored in status.yaml files)
- ✅ WalrusStatsReporter interface for Phase 4 integration
- ✅ Per-workdir and combined stats available
- ✅ Success/failure rate calculations and convenience methods

**Testing after Phase 3:**
- `testnet wal-relay enable/disable/status` commands work correctly
- Status.yaml files contain only status information (no statistics)
- WalrusMonitor provides API access to request statistics
- Request statistics ready for Phase 4 HTTP proxy integration

### Phase 4: HTTP Proxy Implementation ❌ TODO
**Rust files to modify:**
- Extend `rust/suibase/crates/suibase-daemon/src/proxy_server.rs`:
  - Add walrus relay route handling to existing HTTP forwarding logic
  - Forward walrus requests to http://localhost:{local_port}
  - Route `/v1/blob-upload-relay` and other walrus endpoints
- Update proxy server configuration to handle walrus relay routes

**Testing after Phase 4:**
- `curl http://localhost:45852/v1/tip-config` should forward to backend relay
- `curl http://localhost:45852/v1/blob-upload-relay` should work transparently
- All walrus API endpoints should work through the proxy

**Error handling:**
- Network validation (testnet/mainnet only)
- Binary availability checks
- Port conflict detection
- Clear error messages for user

## Configuration

### suibase.yaml
```yaml
walrus_relay_enabled: false
walrus_relay_proxy_port: 45852  # 45853 for mainnet
walrus_relay_local_port: 45802  # 45803 for mainnet
```


## Testing

All walrus relay functionality is tested using the existing test infrastructure at `scripts/tests/050_walrus_tests/`.

### Test Structure ✅ COMPLETE
- Comprehensive test suite in `scripts/tests/050_walrus_tests/`
- Tests cover all core functionality:
  - ✅ Binary installation and management (`test_binary_management.sh`)
  - ✅ Status reporting and health checks (`test_relay_status_integration.sh`)
  - ✅ Enable/disable command functionality (`test_relay_cli_commands.sh`)
  - ✅ Daemon integration and edge cases (`test_daemon_stop_edge_cases.sh`)
  - ✅ Configuration integrity (`test_suibase_yaml_integrity.sh`)

### API Compatibility Test Examples (Phase 4 - TODO)
```bash
# These will work once HTTP proxy implementation is complete:
curl http://localhost:45852/v1/tip-config
curl -X POST http://localhost:45852/v1/blob-upload-relay \
  -H "Content-Type: application/octet-stream" \
  --data-binary @blob.data

# Future endpoints will be automatically supported:
curl http://localhost:45852/v2/new-endpoint
```

## Deployment

- No breaking changes - additive functionality
- Users opt in by adding configuration to `suibase.yaml`
- Default disabled initially

## Future Extensions

The proxy architecture enables future enhancements like pro-tier services, metrics, and smart routing without breaking existing integrations.

## References and Documentation

### Essential Documentation
- **Walrus Upload Relay Guide**: https://docs.wal.app/operator-guide/upload-relay.html
  - Configuration format, command line arguments, tip settings
  - Docker deployment examples

### Source Code Repository
- **Local Walrus Repository**: `~/repos/walrus-reference-main` ⭐
  - Upload relay source: `crates/walrus-upload-relay/`
  - Setup: `scripts/dev/manage-local-repos.sh` (auto-init/update)
- **Remote GitHub Repository**: https://github.com/MystenLabs/walrus
  - Binary releases: https://github.com/MystenLabs/walrus/releases

### Technical Summary
- **Binary Name**: `walrus-upload-relay`
- **Configuration**: Uses `walrus-config.yaml` and `relay-config.yaml`
- **Health Check**: `/v1/tip-config` endpoint returns JSON configuration
- **Port Strategy**: Fully configurable via `--server-address`

### Implementation References
**Local Walrus source code:**
- Upload relay: `~/repos/walrus-reference-main/crates/walrus-upload-relay/`
- Core types: `~/repos/walrus-reference-main/crates/walrus-core/`

**Suibase implementation patterns:**
- `scripts/common/__autocoins.sh` - Command structure model
- `scripts/common/__sui-faucet-process.sh` - Process management pattern
- `scripts/common/__walrus-binaries.sh` - Binary management integration
- `scripts/common/__workdir-exec.sh` - `echo_process()` for status display