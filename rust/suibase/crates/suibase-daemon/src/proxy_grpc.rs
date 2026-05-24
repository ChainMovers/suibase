// gRPC reverse-proxy support for ProxyServer.
//
// Forwards inbound HTTP/2 gRPC requests to an upstream sui-node, preserving
// headers, body, and trailers. Both unary AND server-streaming responses are
// passed through: the response body is wrapped (not collected) so frames flow
// from upstream → proxy → client as they arrive. Client-streaming and
// bidi-streaming remain unsupported (the inbound body is still buffered up
// to MAX_BODY_SIZE before forwarding).
//
// Modeled on the patterns in ~/sui-proxy/crates/sui-proxy (HTTP/2 reverse
// proxy for Sui gRPC).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use http::header;
use http::{Request, Response, StatusCode, Uri};
use http_body::{Body, Frame, SizeHint};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;
use tokio::time::{Instant, Sleep};

/// Body type returned by the proxy.
///
/// Uses a boxed `std::error::Error` so the same `Response<ProxyBody>` can be
/// produced by both the gRPC forwarder (errors originate from `hyper`) and
/// the JSON-RPC path (errors originate from `axum`). Both error types
/// implement `Into<Box<dyn Error + Send + Sync>>`.
pub type ProxyError = Box<dyn std::error::Error + Send + Sync>;
pub type ProxyBody = UnsyncBoxBody<Bytes, ProxyError>;

type ReqBody = UnsyncBoxBody<Bytes, ProxyError>;
type HttpsConnector = hyper_rustls::HttpsConnector<HttpConnector>;

/// Maximum request body size (4 MB). gRPC unary messages are typically small;
/// this guards against adversarial payloads without limiting legitimate use.
const MAX_BODY_SIZE: usize = 4 * 1024 * 1024;

/// Timeout for the upstream HEADERS response (until the proxy knows whether
/// the upstream answered at all). Applied only to the request → response-
/// headers leg; the response body has its own per-frame idle deadline
/// (`STREAM_IDLE_TIMEOUT`) instead of a total-body cap, so legitimate
/// server-streaming RPCs (e.g. SubscribeCheckpoints) can run as long as the
/// upstream keeps emitting frames.
const UPSTREAM_HEADER_TIMEOUT: Duration = Duration::from_secs(30);

/// Per-frame idle deadline on the streamed response body. If no DATA or
/// TRAILERS frame arrives from the upstream within this window the proxy
/// terminates the body with an error (which surfaces as an HTTP/2 stream
/// reset to the client). This bounds the failure mode for a half-closed
/// upstream that sent valid headers but then went silent — without it, a
/// stuck upstream pins the client + proxy task for the full lifetime of
/// the client's own timeout (often hours).
///
/// Picked larger than typical inter-frame gaps for legitimate streams
/// (e.g. SubscribeCheckpoints emits a frame per checkpoint; localnet
/// checkpoint interval is ~1s, mainnet ~2s). 60s is comfortably above
/// that while still catching adversarial silence within a reasonable
/// budget.
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Inbound body read budget. Unary gRPC clients send their (small) message
/// then END_STREAM immediately; this only fires if the client opens a
/// streaming RPC. Kept tight so the connection task doesn't park.
const INBOUND_BODY_TIMEOUT: Duration = Duration::from_secs(5);

/// HTTP/2-capable client shared across requests. Construct once per
/// ProxyServer; clone the Arc into request-handler closures.
#[derive(Clone)]
pub struct GrpcProxyClient {
    inner: Client<HttpsConnector, ReqBody>,
}

impl GrpcProxyClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http2()
            .build();

        let inner = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build(https_connector);

        Ok(Self { inner })
    }
}

/// Case-insensitive check on a content-type string. Media-type matching is
/// case-insensitive per RFC 6838 §4.2.
fn content_type_is_grpc(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 16 && bytes[..16].eq_ignore_ascii_case(b"application/grpc")
}

/// Returns true if the request looks like gRPC (content-type starts with
/// `application/grpc`, case-insensitive). gRPC sub-protocols (grpc+proto,
/// grpc-web, etc.) all share the `application/grpc` prefix.
pub fn is_grpc_request<B>(req: &Request<B>) -> bool {
    req.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(content_type_is_grpc)
        .unwrap_or(false)
}

/// Internal routing/probe headers (set by NetworkMonitor / RequestWorker) that
/// must NEVER leak to an upstream sui-node. Mirrors the explicit removes the
/// JSON-RPC handler does in proxy_server.rs (process_header_server_idx /
/// process_header_server_health_check).
const HEADER_SBSD_SERVER_IDX: &str = "x-sbsd-server-idx";
const HEADER_SBSD_SERVER_HC: &str = "x-sbsd-server-hc";

fn is_internal_routing_header(name: &http::HeaderName) -> bool {
    let n = name.as_str();
    n.eq_ignore_ascii_case(HEADER_SBSD_SERVER_IDX)
        || n.eq_ignore_ascii_case(HEADER_SBSD_SERVER_HC)
}

/// A request that's been fully buffered and is ready to be sent to one or
/// more upstreams. Constructed once via `BufferedGrpcRequest::from_request`
/// so the caller can drive its own per-upstream retry loop (e.g. checking
/// rate limits or per-attempt globals between forwards).
pub struct BufferedGrpcRequest {
    method: http::Method,
    version: http::Version,
    headers: http::HeaderMap,
    body_bytes: Bytes,
    path_and_query: String,
}

impl BufferedGrpcRequest {
    /// Consume an inbound request and buffer its body. Returns either a
    /// usable `BufferedGrpcRequest` or an early-response variant
    /// (e.g. body too large).
    ///
    /// Imposes `INBOUND_BODY_TIMEOUT` on the collect — without it a client
    /// that opens a *client-streaming* or *bidi-streaming* RPC (where the
    /// request body stays open while the client streams messages) would
    /// hang this connection task indefinitely. Server-streaming and unary
    /// RPCs both send their (small) request message followed immediately
    /// by END_STREAM, so this timeout fires only for inbound-streaming
    /// shapes we do not support yet. Note: this is asymmetric with the
    /// response body, which is wrapped in `IdleTimeoutBody` and streamed
    /// through verbatim — see the module header.
    pub async fn from_request(
        req: Request<Incoming>,
    ) -> Result<Self, Response<ProxyBody>> {
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str().to_string())
            .unwrap_or_else(|| "/".to_string());

        let (parts, body) = req.into_parts();
        let method = parts.method;
        let version = parts.version;
        let headers = parts.headers;

        let collect_fut = BodyExt::collect(Limited::new(body, MAX_BODY_SIZE));
        let body_bytes = match tokio::time::timeout(INBOUND_BODY_TIMEOUT, collect_fut).await {
            Ok(Ok(collected)) => collected.to_bytes(),
            Ok(Err(e)) => {
                // http_body_util::Limited yields a boxed error. Walk the
                // source chain to detect LengthLimitError without depending
                // on its Display text.
                if is_length_limit_error(e.as_ref()) {
                    return Err(grpc_resource_exhausted_response("request body too large"));
                }
                log::debug!("grpc: failed to read request body: {}", e);
                return Err(grpc_internal_response("failed to read request body"));
            }
            Err(_) => {
                // Inbound body never reached EOS within the budget — most
                // likely a streaming RPC. Streaming is not yet supported.
                return Err(grpc_unimplemented_response(
                    "streaming gRPC not supported by this proxy",
                ));
            }
        };

        Ok(Self {
            method,
            version,
            headers,
            body_bytes,
            path_and_query,
        })
    }
}

fn is_length_limit_error(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = current {
        if e.downcast_ref::<http_body_util::LengthLimitError>().is_some() {
            return true;
        }
        current = e.source();
    }
    false
}

/// Outcome of forwarding to one upstream.
pub enum ForwardOutcome {
    /// Upstream produced a response that looks like gRPC. Returned to the
    /// client as-is.
    GrpcResponse(Response<ProxyBody>),
    /// Upstream answered HTTP 200 but with a non-gRPC content-type — a
    /// definitive capability signal that the upstream isn't a gRPC server
    /// (e.g. a JSON-RPC-only gateway answering on the gRPC path). Caller
    /// should permanently demote the upstream.
    NotGrpc {
        status: StatusCode,
        content_type: String,
    },
    /// Upstream answered with a non-200 HTTP status (e.g. 429/502/503/504).
    /// This is a transient health/load signal — the upstream MIGHT still be
    /// gRPC-capable. Caller should NOT permanently demote on this alone.
    HttpError {
        status: StatusCode,
        content_type: String,
    },
    /// Upstream couldn't be reached or response read failed.
    Error(String),
}

/// Forward a buffered request to a single upstream. The caller drives the
/// retry loop across multiple upstreams.
pub async fn forward_to_upstream(
    buf: &BufferedGrpcRequest,
    base_url: &str,
    client: Arc<GrpcProxyClient>,
) -> ForwardOutcome {
    try_forward_one(
        base_url,
        &buf.path_and_query,
        &buf.method,
        buf.version,
        &buf.headers,
        buf.body_bytes.clone(),
        client,
    )
    .await
}

/// Build a "no gRPC-capable upstream" response. Public so callers that hit
/// the end of their upstream list can return a parseable gRPC error.
pub fn grpc_no_upstream_response(msg: &str) -> Response<ProxyBody> {
    grpc_unavailable_response(msg)
}

#[allow(clippy::too_many_arguments)]
async fn try_forward_one(
    base_url: &str,
    path_and_query: &str,
    method: &http::Method,
    _version: http::Version,
    headers: &http::HeaderMap,
    body_bytes: Bytes,
    client: Arc<GrpcProxyClient>,
) -> ForwardOutcome {
    log::debug!(
        "grpc: forwarding {} {} → {} ({} bytes)",
        method,
        path_and_query,
        base_url,
        body_bytes.len()
    );
    let upstream_uri = match build_upstream_uri(base_url, path_and_query) {
        Ok(u) => u,
        Err(e) => {
            return ForwardOutcome::Error(format!("invalid upstream URL: {}", e));
        }
    };

    // Don't replay the inbound HTTP version — the upstream `GrpcProxyClient`
    // is `http2_only(true)`, so we always speak HTTP/2 outbound regardless
    // of whether the inbound was HTTP/1.1 (e.g. gRPC-Web) or HTTP/2.
    let mut builder = Request::builder()
        .method(method.clone())
        .uri(upstream_uri);

    // Copy all headers except `host` (let hyper derive it from the new URI)
    // and the internal X-SBSD-* routing headers (must never leak upstream).
    // Preserving `te: trailers` is required by gRPC.
    //
    // Drop any inbound `grpc-accept-encoding` and force `identity` outbound:
    // the Sui CLI's tonic gRPC client (≤ 1.72.x) cannot decode message-level
    // gzip; without this, public upstreams that compress (chainbase /
    // nodeinfra) cause the CLI to fail with
    //   UNIMPLEMENTED: Content is compressed with `gzip` which isn't supported
    // even though the proxy forwarding succeeded. JSON-RPC sunset (July 2026)
    // means the alternative — keeping a JSON-RPC-only sui.io in the link list
    // — has no future; instead we ask every gRPC upstream not to compress.
    //
    // Scope: this is request-side negotiation only. A spec-compliant upstream
    // honors `grpc-accept-encoding: identity` and stops setting the per-message
    // compression-flag byte. The response body is still streamed verbatim
    // (`IdleTimeoutBody` does no per-frame inspection), so a misbehaving
    // upstream that ignored the request header would still produce frames the
    // CLI can't decode. No known Sui upstream does this today.
    for (key, value) in headers.iter() {
        if key == header::HOST || is_internal_routing_header(key) {
            continue;
        }
        if key.as_str().eq_ignore_ascii_case("grpc-accept-encoding") {
            continue;
        }
        builder = builder.header(key, value);
    }
    builder = builder.header("grpc-accept-encoding", "identity");

    let req_body: ReqBody = Full::new(body_bytes)
        .map_err(|never| -> ProxyError { match never {} })
        .boxed_unsync();

    let upstream_req = match builder.body(req_body) {
        Ok(r) => r,
        Err(e) => return ForwardOutcome::Error(format!("build request: {}", e)),
    };

    let header_result =
        tokio::time::timeout(UPSTREAM_HEADER_TIMEOUT, client.inner.request(upstream_req)).await;

    let resp = match header_result {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return ForwardOutcome::Error(format!("connect: {}", e)),
        Err(_) => return ForwardOutcome::Error("header timeout".to_string()),
    };

    let status = resp.status();
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let ct_is_grpc = content_type_is_grpc(&content_type);
    log::debug!(
        "grpc: upstream response {} for {} (status={}, content-type={})",
        path_and_query,
        base_url,
        status,
        content_type
    );

    // Three buckets:
    //   * 200 + application/grpc           → real gRPC response (forward)
    //   * 200 + other content-type         → capability signal (permanent demote)
    //   * non-200 (any content-type)       → transient HTTP error (do NOT demote)
    if status != StatusCode::OK {
        return ForwardOutcome::HttpError {
            status,
            content_type,
        };
    }
    if !ct_is_grpc {
        return ForwardOutcome::NotGrpc {
            status,
            content_type,
        };
    }

    // Stream the response body through to the client. We deliberately do NOT
    // collect/buffer it: server-streaming RPCs (e.g. SubscribeCheckpoints)
    // have bodies that may run for hours. Hyper's body type carries frames
    // (DATA and TRAILERS) verbatim, so grpc-status arrives at the client
    // naturally as part of the trailing HEADERS frame.
    //
    // The body is wrapped in `IdleTimeoutBody` so a half-closed upstream
    // (sent valid headers, then went silent) is detected within
    // STREAM_IDLE_TIMEOUT and the body terminates with an error instead
    // of pinning the client + proxy task indefinitely. Upstream body
    // errors are also logged at debug from the wrapper so operators
    // running with RUST_LOG=debug can see mid-stream failures.
    let (resp_parts, resp_body) = resp.into_parts();
    let log_target = format!("{} for {}", path_and_query, base_url);
    let wrapped = IdleTimeoutBody::new(resp_body, STREAM_IDLE_TIMEOUT, log_target);
    let boxed: ProxyBody = wrapped.boxed_unsync();
    ForwardOutcome::GrpcResponse(Response::from_parts(resp_parts, boxed))
}

/// `Body` adapter that imposes a per-frame idle deadline on the wrapped
/// upstream body. If no frame (DATA or TRAILERS) arrives within
/// `timeout` the wrapper terminates the body with an error; otherwise
/// it forwards each frame verbatim. The idle timer is reset on each
/// frame so legitimate streaming RPCs whose inter-frame gaps are
/// shorter than `timeout` are never interrupted.
///
/// Upstream body errors are logged at debug level (with the originating
/// path + upstream as context) so operators running with
/// `RUST_LOG=debug` can observe mid-stream failures — these errors
/// can't be reported back through `req_resp_ok` (which has already
/// fired by the time the body starts streaming) but the log entry is
/// the per-stream signal NetworkMonitor can't currently capture.
struct IdleTimeoutBody<B> {
    inner: B,
    timeout: Duration,
    // `Sleep` is `!Unpin`; we keep it on the heap to make `IdleTimeoutBody`
    // itself easy to manually pin-project.
    timer: Pin<Box<Sleep>>,
    log_target: String,
    failed: bool,
}

impl<B> IdleTimeoutBody<B> {
    fn new(inner: B, timeout: Duration, log_target: String) -> Self {
        Self {
            inner,
            timeout,
            timer: Box::pin(tokio::time::sleep(timeout)),
            log_target,
            failed: false,
        }
    }
}

impl<B> Body for IdleTimeoutBody<B>
where
    B: Body<Data = Bytes>,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    type Data = Bytes;
    type Error = ProxyError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // SAFETY: `inner` (Incoming), `timeout`, `log_target`, and
        // `failed` are never moved through this projection. The only
        // `!Unpin` field, `Sleep`, is already heap-pinned in
        // `Pin<Box<Sleep>>` so we can poll it through a mutable
        // borrow on the Box without violating the pin contract.
        let this = unsafe { self.get_unchecked_mut() };

        if this.failed {
            // Already errored once; do not poll further (avoids
            // emitting multiple error frames or polling a hyper body
            // after it surfaced an error).
            return Poll::Ready(None);
        }

        if this.timer.as_mut().poll(cx).is_ready() {
            log::debug!(
                "grpc: response body idle for {:?} on {} — terminating stream",
                this.timeout,
                this.log_target
            );
            this.failed = true;
            return Poll::Ready(Some(Err(format!(
                "upstream body idle for {:?}",
                this.timeout
            )
            .into())));
        }

        // SAFETY: `inner` is structurally pinned via this projection
        // and never moved out of `self`.
        let inner_pin = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner_pin.poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                // Reset the idle timer on each frame received.
                let new_deadline = Instant::now() + this.timeout;
                this.timer.as_mut().reset(new_deadline);
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(e))) => {
                log::debug!(
                    "grpc: response body error on {}: {}",
                    this.log_target,
                    e
                );
                this.failed = true;
                Poll::Ready(Some(Err(Box::new(e))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.failed || self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

/// Build a gRPC-shaped response with the given status code so the client gets
/// a parseable gRPC status instead of a cryptic HTTP body.
///
/// Uses the "trailers-only" form: status code in regular headers, empty body.
/// This is the canonical way to express an early failure in gRPC over HTTP/2.
fn grpc_status_response(grpc_status: u8, msg: &str) -> Response<ProxyBody> {
    // grpc-message is percent-encoded; keep it ASCII-safe by stripping
    // anything that would need escaping rather than pulling in a urlencoder.
    let safe: String = msg
        .chars()
        .map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '_' })
        .collect();
    let body = full_body("");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/grpc")
        .header("grpc-status", grpc_status.to_string())
        .header("grpc-message", safe)
        .body(body)
        .unwrap()
}

fn grpc_unavailable_response(msg: &str) -> Response<ProxyBody> {
    grpc_status_response(14, msg) // UNAVAILABLE
}

fn grpc_resource_exhausted_response(msg: &str) -> Response<ProxyBody> {
    grpc_status_response(8, msg) // RESOURCE_EXHAUSTED
}

fn grpc_internal_response(msg: &str) -> Response<ProxyBody> {
    grpc_status_response(13, msg) // INTERNAL
}

fn grpc_unimplemented_response(msg: &str) -> Response<ProxyBody> {
    grpc_status_response(12, msg) // UNIMPLEMENTED
}

/// Public version of `grpc_unimplemented_response` for proxy_server use.
pub fn grpc_unimplemented(msg: &str) -> Response<ProxyBody> {
    grpc_unimplemented_response(msg)
}

fn build_upstream_uri(base: &str, path_and_query: &str) -> Result<Uri, ProxyError> {
    let base_uri: Uri = base.parse().map_err(|e| -> ProxyError { Box::new(e) })?;
    let authority = base_uri
        .authority()
        .ok_or_else(|| -> ProxyError {
            format!("upstream URL '{}' has no authority (host:port)", base).into()
        })?
        .clone();
    Uri::builder()
        .scheme(base_uri.scheme_str().unwrap_or("http"))
        .authority(authority)
        .path_and_query(path_and_query)
        .build()
        .map_err(|e| -> ProxyError { Box::new(e) })
}

fn full_body(s: &str) -> ProxyBody {
    Full::new(Bytes::from(s.to_string()))
        .map_err(|never| -> ProxyError { match never {} })
        .boxed_unsync()
}

/// Public gRPC-shaped UNAVAILABLE response for the proxy_server dispatch path.
/// The `_status` parameter is preserved for backward-compat with existing
/// callers but the response is always a parseable gRPC trailers-only frame
/// (HTTP 200, content-type: application/grpc, grpc-status: 14 UNAVAILABLE).
pub fn error_proxy_response(_status: StatusCode, msg: &str) -> Response<ProxyBody> {
    grpc_unavailable_response(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::HeaderValue;

    fn req_with_ct(ct: &str) -> Request<()> {
        let mut r = Request::builder().body(()).unwrap();
        r.headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_str(ct).unwrap());
        r
    }

    #[test]
    fn detects_grpc_canonical() {
        assert!(is_grpc_request(&req_with_ct("application/grpc")));
    }

    #[test]
    fn detects_grpc_with_subtype() {
        assert!(is_grpc_request(&req_with_ct("application/grpc+proto")));
        assert!(is_grpc_request(&req_with_ct("application/grpc-web")));
        assert!(is_grpc_request(&req_with_ct("application/grpc-web+proto")));
    }

    #[test]
    fn rejects_json_content_type() {
        assert!(!is_grpc_request(&req_with_ct("application/json")));
        assert!(!is_grpc_request(&req_with_ct("text/plain")));
    }

    #[test]
    fn rejects_missing_content_type() {
        let r: Request<()> = Request::builder().body(()).unwrap();
        assert!(!is_grpc_request(&r));
    }

    // RFC 6838 §4.2 says media-type subtype matching is case-insensitive. A
    // misbehaving intermediary that normalizes the casing must not cause a
    // valid gRPC request to be dispatched to the JSON-RPC handler.
    #[test]
    fn detects_grpc_case_insensitive() {
        assert!(is_grpc_request(&req_with_ct("Application/grpc")));
        assert!(is_grpc_request(&req_with_ct("APPLICATION/GRPC")));
        assert!(is_grpc_request(&req_with_ct("Application/grpc+Proto")));
        assert!(is_grpc_request(&req_with_ct("application/GRPC-web")));
    }

    #[test]
    fn looks_like_grpc_case_insensitive() {
        assert!(content_type_is_grpc("application/grpc"));
        assert!(content_type_is_grpc("Application/grpc"));
        assert!(content_type_is_grpc("APPLICATION/GRPC+PROTO"));
        assert!(!content_type_is_grpc("application/json"));
        assert!(!content_type_is_grpc(""));
    }

    #[test]
    fn builds_upstream_uri_preserves_path() {
        let uri = build_upstream_uri("http://127.0.0.1:9000", "/sui.rpc.v2.LedgerService/GetCheckpoint")
            .expect("build should succeed");
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().map(|a| a.as_str()), Some("127.0.0.1:9000"));
        assert_eq!(uri.path(), "/sui.rpc.v2.LedgerService/GetCheckpoint");
    }

    #[test]
    fn builds_upstream_uri_https_scheme() {
        let uri = build_upstream_uri("https://example.com:443", "/x").expect("ok");
        assert_eq!(uri.scheme_str(), Some("https"));
    }

    #[test]
    fn builds_upstream_uri_rejects_no_authority() {
        // bare path has no authority -> error
        assert!(build_upstream_uri("/just/a/path", "/x").is_err());
    }

    // ----- IdleTimeoutBody tests -----
    //
    // The wrapper is generic over its inner body so we can plug in a
    // controlled test body without needing a real hyper Incoming.

    use futures::stream;
    use http_body_util::StreamBody;
    use std::convert::Infallible;

    /// Build a test body that emits `n` empty DATA frames, each
    /// `delay` apart. After the n-th frame the body ends (poll_frame
    /// → Ready(None)). The body type is `StreamBody` over a
    /// `futures::Stream` so it has a stable type that satisfies
    /// `Body<Data = Bytes>` with `Error = Infallible`.
    fn pulsing_body(
        n: usize,
        delay: Duration,
    ) -> StreamBody<impl futures::Stream<Item = Result<Frame<Bytes>, Infallible>>> {
        let s = stream::unfold(0usize, move |i| async move {
            if i >= n {
                None
            } else {
                tokio::time::sleep(delay).await;
                Some((
                    Ok::<_, Infallible>(Frame::data(Bytes::from_static(&[0u8, 0, 0, 0, 0]))),
                    i + 1,
                ))
            }
        });
        StreamBody::new(s)
    }

    /// Drain a body to completion (or its first error) and return
    /// (frames_received, terminating_error_if_any).
    async fn drain_body<B>(body: B) -> (usize, Option<String>)
    where
        B: Body<Data = Bytes>,
        B::Error: std::fmt::Display,
    {
        let mut pinned = Box::pin(body);
        let mut count = 0usize;
        loop {
            match pinned.as_mut().frame().await {
                Some(Ok(_frame)) => count += 1,
                Some(Err(e)) => return (count, Some(e.to_string())),
                None => return (count, None),
            }
        }
    }

    #[tokio::test]
    async fn idle_timeout_fires_when_inter_frame_gap_exceeds_budget() {
        // Inner emits 2 frames with a 300ms gap; idle timeout 100ms.
        // The idle timer MUST fire before the first frame arrives,
        // terminating the body with an error. We allow up to 1 frame
        // through in case the runtime polls the inner future before
        // the timer (depends on tokio's queue order).
        let inner = pulsing_body(2, Duration::from_millis(300));
        let wrapper = IdleTimeoutBody::new(
            inner,
            Duration::from_millis(100),
            "test-target".to_string(),
        );
        let (frames, err) = drain_body(wrapper).await;
        assert!(
            err.is_some(),
            "expected idle-timeout error, got frames={} err=None",
            frames
        );
        let msg = err.unwrap();
        assert!(
            msg.contains("idle"),
            "error message should mention 'idle', got: {}",
            msg
        );
        assert!(
            frames <= 1,
            "expected at most 1 frame before timeout, got {}",
            frames
        );
    }

    #[tokio::test]
    async fn idle_timer_resets_on_each_frame() {
        // Inner emits 5 frames at a 50ms cadence; idle timeout 300ms.
        // Each frame should reset the timer, so all 5 must pass
        // through without error. This is the legitimate-streaming
        // case the timeout is calibrated NOT to interfere with.
        let inner = pulsing_body(5, Duration::from_millis(50));
        let wrapper = IdleTimeoutBody::new(
            inner,
            Duration::from_millis(300),
            "test-target".to_string(),
        );
        let (frames, err) = drain_body(wrapper).await;
        assert_eq!(
            frames, 5,
            "all 5 frames must pass through (timer resets per frame); err={:?}",
            err
        );
        assert!(
            err.is_none(),
            "legitimate streaming must not produce an error; got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn body_completing_cleanly_is_not_misclassified_as_timeout() {
        // Inner emits 2 frames quickly then ends. Idle timeout
        // generous (300ms). Wrapper must return frames + clean end,
        // never producing a spurious timeout error.
        let inner = pulsing_body(2, Duration::from_millis(20));
        let wrapper = IdleTimeoutBody::new(
            inner,
            Duration::from_millis(300),
            "test-target".to_string(),
        );
        let (frames, err) = drain_body(wrapper).await;
        assert_eq!(frames, 2, "both frames must pass through");
        assert!(
            err.is_none(),
            "clean stream end must not be mis-reported as timeout: {:?}",
            err
        );
    }

    /// Build a test body that emits one DATA frame then a TRAILERS
    /// frame carrying `grpc-status: 0`. Used to verify that the
    /// wrapper preserves HTTP/2 TRAILERS (where gRPC delivers its
    /// status code). Without this test, a regression that strips
    /// trailers (e.g. re-introducing a `body.collect()` style
    /// buffering layer that ignores trailer frames) would not be
    /// caught — tonic clients hang on `recv()` waiting for a
    /// terminating status that never arrives.
    fn body_with_trailers(
    ) -> StreamBody<impl futures::Stream<Item = Result<Frame<Bytes>, Infallible>>> {
        use http::HeaderMap;
        let frames: Vec<Result<Frame<Bytes>, Infallible>> = {
            let mut trailers = HeaderMap::new();
            trailers.insert("grpc-status", "0".parse().unwrap());
            vec![
                Ok(Frame::data(Bytes::from_static(&[0u8, 0, 0, 0, 0]))),
                Ok(Frame::trailers(trailers)),
            ]
        };
        StreamBody::new(stream::iter(frames))
    }

    #[tokio::test]
    async fn trailers_are_forwarded_verbatim_through_the_wrapper() {
        // The wrapper must pass HTTP/2 TRAILERS frames through
        // verbatim — gRPC delivers `grpc-status` in trailers, so a
        // proxy that drops trailer frames leaves real clients
        // hanging on recv().
        let wrapper = IdleTimeoutBody::new(
            body_with_trailers(),
            Duration::from_secs(5),
            "test-target".to_string(),
        );
        let mut pinned = Box::pin(wrapper);
        let mut data_frames = 0usize;
        let mut grpc_status: Option<String> = None;
        while let Some(frame_result) = pinned.as_mut().frame().await {
            let frame = frame_result.expect("frame should not error");
            if frame.is_data() {
                data_frames += 1;
            } else if frame.is_trailers() {
                let trailers = frame.into_trailers().ok().expect("trailers");
                if let Some(v) = trailers.get("grpc-status") {
                    grpc_status = Some(v.to_str().unwrap().to_string());
                }
            }
        }
        assert_eq!(data_frames, 1, "DATA frame must pass through");
        assert_eq!(
            grpc_status.as_deref(),
            Some("0"),
            "TRAILERS frame with grpc-status must pass through"
        );
    }
}
