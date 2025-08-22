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
testnet walrus-relay status    # Show current status
testnet walrus-relay enable    # Enable relay proxy
testnet walrus-relay disable   # Disable relay proxy
mainnet walrus-relay status    # Same for mainnet
mainnet walrus-relay enable
mainnet walrus-relay disable
```

Status display format (using existing echo_process function):
```
Walrus Relay     : OK ( pid 1223131 ) http://localhost:45852
Walrus Relay     : DISABLED
Walrus Relay     : INITIALIZING ( pid 1223131 ) http://localhost:45852  
Walrus Relay     : NOT RUNNING http://localhost:45852
```
- Uses 17-character left-aligned label (same as "Proxy server")
- PID is the walrus-upload-relay backend process (not suibase-daemon)
- URL shows suibase-daemon proxy port (where users connect)

## Implementation Phases

### Phase 1: Binary Process Management
**Process lifecycle setup:**
- Extend `scripts/common/__walrus-binaries.sh`:
  - Add walrus-upload-relay to binary management
  - Download binary via existing walrus management
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
- Log location: `~/suibase/workdirs/{testnet,mainnet}/walrus-relay-process.log`
- Hook into existing `testnet update` and `mainnet update` commands

### Phase 2: Bash Scripts and Command Integration
**Files to create/modify:**
- Create `scripts/common/__walrus-relay.sh` with:
  - `walrus_relay_status()` - Use existing echo_process() with relay PID and proxy URL
  - `walrus_relay_enable()` - Set walrus_relay.enabled: true in suibase.yaml
  - `walrus_relay_disable()` - Set walrus_relay.enabled: false in suibase.yaml
  - `update_walrus_relay_status_yaml()` - Parse status.yaml from daemon
- Modify `scripts/common/__workdir-exec.sh`:
  - Add `walrus-relay) CMD_WALRUS_RELAY_REQ=true ;;` to command parsing
  - Add subcommand parsing for status|enable|disable
  - Add execution block calling walrus_relay_* functions

**Configuration in suibase.yaml:**
```yaml
walrus_relay:
  enabled: false
  proxy_port: 45852  # 45853 for mainnet
  local_port: 45802  # 45803 for mainnet
```

**Testing after Phase 1:**
- `testnet walrus-relay enable` should start walrus-upload-relay process
- `testnet walrus-relay status` should show relay PID and port  
- Health check should work: `curl http://localhost:45802/v1/tip-config` (returns JSON)
- API discovery should work: `curl http://localhost:45802/v1/api` (returns API spec)

### Phase 2: Suibase-daemon Proxy Implementation
**Rust files to modify:**
- Modify `rust/suibase/crates/suibase-daemon/src/admin_controller.rs`:
  - Detect walrus_relay.enabled changes in suibase.yaml
  - Configure existing ProxyServer instances for walrus relay endpoints
  - Write status to `workdirs/{testnet,mainnet}/walrus-relay/status.yaml`
- Extend `rust/suibase/crates/suibase-daemon/src/proxy_server.rs`:
  - Add walrus relay route handling to existing HTTP forwarding logic
  - Forward walrus requests to http://localhost:{local_port}

**Status management:**
- Daemon writes: `~/suibase/workdirs/{testnet,mainnet}/walrus-relay/status.yaml`
- Status values: DISABLED, INITIALIZING, OK, DOWN
- Include backend connectivity info

**Testing after Phase 2:**
- `curl http://localhost:45852/v1/tip-config` should forward to backend relay
- Proxy should work transparently for all endpoints

**Error handling:**
- Network validation (testnet/mainnet only)
- Binary availability checks
- Port conflict detection
- Clear error messages for user

## Configuration

### suibase.yaml
```yaml
walrus_relay:
  enabled: false
  proxy_port: 45852  # 45853 for mainnet  
  local_port: 45802  # 45803 for mainnet
```


## Testing

All walrus relay functionality should be tested using the existing test infrastructure at `scripts/tests/`. Create a new test directory `scripts/tests/050_walrus_tests/` following the established patterns.

### Test Structure
- Follow existing test conventions in `scripts/tests/`
- Create `scripts/tests/050_walrus_tests/` directory
- Include automated tests for:
  - Walrus relay binary installation and management
  - Proxy forwarding functionality  
  - Status reporting and health checks
  - Enable/disable command functionality

### API Compatibility Test Examples
```bash
# Any HTTP request works - examples:
curl http://localhost:45852/v1/tip-config
curl -X POST http://localhost:45852/v1/blob-upload-relay \
  -H "Content-Type: application/octet-stream" \
  --data-binary @blob.data
  
# Future endpoints automatically supported:
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