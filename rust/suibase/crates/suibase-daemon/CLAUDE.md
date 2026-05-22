# suibase-daemon

gRPC + JSON-RPC proxy server and workdir orchestrator for Suibase.

## Architecture

**Entry**: `main.rs` → `AdminController` (orchestrator) → subsystems

**Key Tasks**:
- `AdminController`: Config manager, starts/stops per-workdir services
- `NetworkMonitor`: Tracks RPC server health/latency
- `ProxyServer`: Per-port async proxy (hyper-util listener; dispatches between
  gRPC unary forwarder and JSON-RPC axum handler by `content-type`)
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
- Per-workdir proxy listener + dispatch: `src/proxy_server.rs`
- gRPC unary forwarder: `src/proxy_grpc.rs`

## gRPC support

The same listener port (e.g. `44340` for localnet) accepts both HTTP/1.1
JSON-RPC and HTTP/2 gRPC. `hyper-util`'s `auto::Builder` negotiates protocol
from the client preface. The request handler:

- `application/grpc*` → `proxy_grpc::forward_unary` (HTTP/2, preserves trailers)
- anything else      → existing axum JSON-RPC handler

**What works**: gRPC unary requests (e.g. `lsui client gas`) — proxy picks the
first healthy target server's RPC URL and forwards the request transparently,
preserving headers + body + trailers.

**Intentionally not yet implemented**:
- Server-streaming RPCs (e.g. `SubscribeCheckpoints`)
- Multi-upstream failover for gRPC (currently picks one target; JSON-RPC path
  retains its existing failover/retry/rate-limit logic)
- gRPC request reporting to `NetworkMonitor` (no stats accumulated yet for
  gRPC traffic; JSON-RPC reporting unchanged)

Reference implementation (very similar pattern, full multi-upstream + streaming):
`~/sui-proxy/crates/sui-proxy/src/proxy.rs`.

## Testing

```bash
cd rust/suibase && cargo test
```

## Notes

- One `InputPort`/`ProxyServer` per workdir (never deleted)
- Hot-reload suibase.yaml via `WorkdirsWatcher`
- WebSocket event dedup in `EventsWriterWorker`