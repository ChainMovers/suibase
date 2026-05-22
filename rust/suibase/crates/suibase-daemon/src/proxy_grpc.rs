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

/// Returns true if the request looks like gRPC (content-type starts with
/// `application/grpc`). gRPC sub-protocols (grpc+proto, grpc-web, etc.) all
/// share the `application/grpc` prefix.
pub fn is_grpc_request<B>(req: &Request<B>) -> bool {
    req.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.starts_with("application/grpc"))
        .unwrap_or(false)
}

/// Forward a single gRPC unary request to `upstream_rpc_url`, returning the
/// response (headers + body + trailers preserved).
///
/// `upstream_rpc_url` is the base URL of the sui-node RPC endpoint
/// (e.g. `http://0.0.0.0:9000`); the request's path-and-query is preserved.
pub async fn forward_unary(
    req: Request<Incoming>,
    upstream_rpc_url: &str,
    client: Arc<GrpcProxyClient>,
) -> Result<Response<ProxyBody>, hyper::Error> {
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());

    let upstream_uri = match build_upstream_uri(upstream_rpc_url, &path_and_query) {
        Ok(u) => u,
        Err(e) => {
            log::warn!(
                "grpc: failed to build upstream URI from base '{}' path '{}': {}",
                upstream_rpc_url,
                path_and_query,
                e
            );
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                "invalid upstream URL",
            ));
        }
    };

    // Buffer body to support retries in the future. For now we only forward
    // once but the buffering also bounds memory.
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let version = parts.version;
    let headers = parts.headers;

    let body_bytes = match BodyExt::collect(Limited::new(body, MAX_BODY_SIZE)).await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("length limit exceeded") {
                return Ok(error_response(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "request body too large",
                ));
            }
            log::debug!("grpc: failed to read request body: {}", err_str);
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "failed to read request body",
            ));
        }
    };

    let mut builder = Request::builder()
        .method(method)
        .uri(upstream_uri)
        .version(version);

    // Copy all headers except `host` (let hyper derive it from the new URI).
    // Preserving `te: trailers` is required by gRPC.
    for (key, value) in headers.iter() {
        if key == header::HOST {
            continue;
        }
        builder = builder.header(key, value);
    }

    let req_body: ReqBody = Full::new(body_bytes)
        .map_err(|never| -> ProxyError { match never {} })
        .boxed_unsync();

    let upstream_req = match builder.body(req_body) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("grpc: failed to build upstream request: {}", e);
            return Ok(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to build upstream request",
            ));
        }
    };

    let header_result =
        tokio::time::timeout(UPSTREAM_TIMEOUT, client.inner.request(upstream_req)).await;

    let resp = match header_result {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            log::warn!("grpc: upstream request failed: {}", e);
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                "upstream unavailable",
            ));
        }
        Err(_) => {
            log::warn!(
                "grpc: upstream header timeout ({}s)",
                UPSTREAM_TIMEOUT.as_secs()
            );
            return Ok(error_response(
                StatusCode::GATEWAY_TIMEOUT,
                "upstream header timeout",
            ));
        }
    };

    // Buffer the response body. `BodyExt::collect` preserves HTTP/2 trailers,
    // which is how gRPC delivers its status code (`grpc-status`).
    let (resp_parts, resp_body) = resp.into_parts();
    let body_result = tokio::time::timeout(UPSTREAM_TIMEOUT, BodyExt::collect(resp_body)).await;

    match body_result {
        Ok(Ok(collected)) => {
            // `Collected<Bytes>::Error` is `Infallible` (collect already
            // succeeded). Map it into our shared error type.
            let boxed: ProxyBody = collected
                .map_err(|never| -> ProxyError { match never {} })
                .boxed_unsync();
            Ok(Response::from_parts(resp_parts, boxed))
        }
        Ok(Err(e)) => {
            log::warn!("grpc: upstream body error: {}", e);
            Ok(error_response(
                StatusCode::BAD_GATEWAY,
                "upstream body error",
            ))
        }
        Err(_) => {
            log::warn!(
                "grpc: upstream body timeout ({}s)",
                UPSTREAM_TIMEOUT.as_secs()
            );
            Ok(error_response(
                StatusCode::GATEWAY_TIMEOUT,
                "upstream body timeout",
            ))
        }
    }
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

fn error_response(status: StatusCode, msg: &str) -> Response<ProxyBody> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(full_body(msg))
        .unwrap()
}

/// Public version for use from the proxy_server dispatch path.
pub fn error_proxy_response(status: StatusCode, msg: &str) -> Response<ProxyBody> {
    error_response(status, msg)
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
