// gRPC reverse-proxy support for ProxyServer.
//
// Forwards inbound HTTP/2 gRPC unary requests to an upstream sui-node,
// preserving headers, body, and trailers. Modeled on the patterns in
// ~/sui-proxy/crates/sui-proxy (HTTP/2 reverse proxy for Sui gRPC).
//
// Streaming RPCs are not yet supported — they short-circuit to an error
// response. That's tracked separately; see the project notes.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::header;
use http::{Request, Response, StatusCode, Uri};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;

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

/// Timeout for a single forwarded upstream request. Long enough for legitimate
/// long-running unary calls; short enough that a dead upstream isn't held open
/// forever.
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(30);

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
    /// Imposes `INBOUND_BODY_TIMEOUT` on the collect — without it, a client
    /// that opens a streaming RPC (which never sends end-of-stream on its own)
    /// would hang the proxy connection task indefinitely. Streaming is not
    /// yet supported; this timeout converts the hang into a parseable gRPC
    /// status the client can react to.
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
    for (key, value) in headers.iter() {
        if key == header::HOST || is_internal_routing_header(key) {
            continue;
        }
        builder = builder.header(key, value);
    }

    let req_body: ReqBody = Full::new(body_bytes)
        .map_err(|never| -> ProxyError { match never {} })
        .boxed_unsync();

    let upstream_req = match builder.body(req_body) {
        Ok(r) => r,
        Err(e) => return ForwardOutcome::Error(format!("build request: {}", e)),
    };

    let header_result =
        tokio::time::timeout(UPSTREAM_TIMEOUT, client.inner.request(upstream_req)).await;

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

    // Buffer the response body. `BodyExt::collect` preserves HTTP/2 trailers,
    // which is how gRPC delivers its status code (`grpc-status`).
    let (resp_parts, resp_body) = resp.into_parts();
    let body_result = tokio::time::timeout(UPSTREAM_TIMEOUT, BodyExt::collect(resp_body)).await;

    match body_result {
        Ok(Ok(collected)) => {
            let boxed: ProxyBody = collected
                .map_err(|never| -> ProxyError { match never {} })
                .boxed_unsync();
            ForwardOutcome::GrpcResponse(Response::from_parts(resp_parts, boxed))
        }
        Ok(Err(e)) => ForwardOutcome::Error(format!("response body: {}", e)),
        Err(_) => ForwardOutcome::Error("body timeout".to_string()),
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
}
