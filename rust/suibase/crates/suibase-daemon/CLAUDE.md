# suibase-daemon

JSON-RPC proxy server and workdir orchestrator for Suibase.

## Architecture

**Entry**: `main.rs` → `AdminController` (orchestrator) → subsystems

**Key Tasks**:
- `AdminController`: Config manager, starts/stops per-workdir services
- `NetworkMonitor`: Tracks RPC server health/latency
- `ProxyServer`: Per-port async HTTP proxy (axum)
- `ClockTrigger`: Periodic audit events
- Workers: `WebSocketWorker`, `DBWorker`, `CliPoller`, `ShellWorker`

**Concurrency**: Tokio async, `Arc<RwLock>` globals, auto-restart on panic via tokio-graceful-shutdown

## Message Protocol

All threads handle 3 event types:
- `EVENT_AUDIT`: Fast read-only consistency check
- `EVENT_UPDATE`: Apply shared state changes
- `EVENT_EXEC`: Execute reactive commands

## Key Paths

- API definitions: `src/api/def_methods.rs`
- API impl: `src/api/impl_{general,proxy,packages}_api.rs`
- Shared state: `src/shared_types/globals.rs`
- Per-workdir RPC proxy: `src/proxy_server.rs`

## Testing

```bash
cd rust/suibase && cargo test
```

## Notes

- One `InputPort`/`ProxyServer` per workdir (never deleted)
- Hot-reload suibase.yaml via `WorkdirsWatcher`
- WebSocket event dedup in `EventsWriterWorker`