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

- `application/grpc*` → `ProxyServer::grpc_dispatch`. It classifies the request
  by method (`proxy_grpc::request_is_bufferable`):
    - whitelisted single-request methods (`sui.rpc.v*`, `grpc.health.*`) are
      buffered and forwarded via `forward_to_upstream`, iterating the
      prioritized upstream list (retry across upstreams).
    - everything else (gRPC server reflection, unknown methods) goes to
      `grpc_dispatch_streaming` → `forward_stream_to_upstream`: the request
      body is piped live to a single upstream (no retry). HTTP/2, preserves
      trailers both directions.
- anything else      → existing axum JSON-RPC handler

**What works**:
- gRPC unary requests (e.g. `lsui client gas`) — proxy picks the healthiest
  selectable target's RPC URL and forwards the request transparently,
  preserving headers + body + trailers.
- gRPC server-streaming responses (e.g. `SubscribeCheckpoints`) — the
  response body is wrapped in `IdleTimeoutBody` and streamed verbatim
  from upstream → client; idle-frame deadline is 60 s.
- gRPC server reflection + client/bidi-streaming requests (e.g. `grpcurl`,
  grpcui, Postman) — non-whitelisted methods have their request body piped
  live to one upstream (`request_is_bufferable` → false → `grpc_dispatch_streaming`).
  Classification is by method path because streaming-ness is a `.proto`
  property, not visible on the wire.
- gRPC fallback across upstreams when the preferred one isn't gRPC-capable
  — `select_grpc_upstreams` builds a prioritized list filtered by
  `selectable: true` (matches JSON-RPC semantics). The buffered path retries
  down this list; the streaming path uses its head (`upstreams.first()`) —
  the first selectable, healthy, gRPC-capable upstream — and commits to it.

**Known limitations**:
- The streaming path uses `upstreams.first()` — the first selectable, healthy,
  gRPC-capable upstream (same #1 the buffered path tries first) — and commits
  to it with no cross-upstream retry, because a half-sent request body can't be
  replayed. This is the right target in practice: localnet has a single
  gRPC-capable node, and the testnet/mainnet default link lists are curated
  gRPC-capable nodes. The only residual edge — an operator's mixed list whose
  healthy #1 happens to be JSON-RPC-only — self-heals: a probe (a buffered
  method) marks it `NOT_GRPC_CAPABLE`, dropping it from the gRPC-capable filter
  so the next request picks the right upstream.
- A whitelisted method whose request body unexpectedly stays open past
  `STREAM_DETECT_GRACE` (250 ms — none do today) falls back to the streaming
  path (logged at debug), losing retry. Real unary clients send END_STREAM
  back-to-back so never hit this.
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