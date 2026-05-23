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

**What works**:
- gRPC unary requests (e.g. `lsui client gas`) — proxy picks the healthiest
  selectable target's RPC URL and forwards the request transparently,
  preserving headers + body + trailers.
- gRPC server-streaming responses (e.g. `SubscribeCheckpoints`) — the
  response body is wrapped in `IdleTimeoutBody` and streamed verbatim
  from upstream → client; idle-frame deadline is 60 s.
- gRPC fallback across upstreams when the preferred one isn't gRPC-capable
  — `select_grpc_upstreams` builds a prioritized list filtered by
  `selectable: true` (matches JSON-RPC semantics).

**Intentionally not yet implemented**:
- Client-streaming and bidi-streaming RPCs — `BufferedGrpcRequest::from_request`
  applies `INBOUND_BODY_TIMEOUT = 5 s` and rejects inbound bodies that don't
  reach END_STREAM in time. Real client-streaming would require buffering or
  a separate inbound-stream wrapper.
- Full body-side observability into `NetworkMonitor`: `req_resp_ok` fires
  when response HEADERS arrive (not when the body completes), so latency
  stats reflect time-to-first-byte rather than time-to-final-trailer.
  Acceptable trade-off for streaming RPCs where the body never ends, but
  worth knowing when comparing gRPC-era and JSON-RPC-era RespT.
- Mid-stream upstream errors surface as HTTP/2 stream resets (no
  grpc-status trailer). The proxy logs them at debug from
  `IdleTimeoutBody::poll_frame` so operators with `RUST_LOG=debug` can
  see them, but the wire-level error class the client receives is a
  transport reset, not a gRPC status code.

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