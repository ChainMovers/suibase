// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! `sb-local` ("suibase localnet") — a standalone, long-running, **localnet-only**
//! HTTP server that exposes the **Walrus aggregator + publisher wire API**, backed by
//! the nodeless [`LocalnetMockStore`]. It is a drop-in replacement for the real
//! `walrus daemon` (combined aggregator + publisher) for localnet clients: point any
//! existing Walrus HTTP client (curl, fetch, walrus-sites) at sb-local by changing
//! only the URL.
//!
//! Topology mirrors the real `daemon` (one process, one port, one router carrying both
//! verbs):
//!   - `GET  /v1/blobs/{blob_id}`                      aggregator read (raw bytes)
//!   - `GET  /v1/blobs/by-object-id/{object_id}`       aggregator read by Sui object id
//!   - `PUT  /v1/blobs`                                publisher store (`BlobStoreResult`)
//!   - `PUT  /v1/quilts`                               publisher quilt store (M5)
//!   - `GET  /v1/blobs/by-quilt-patch-id/{id}`         quilt patch read (M5)
//!   - `GET  /v1/blobs/by-quilt-id/{quilt_id}/{ident}` quilt blob read by identifier (M5)
//!   - `GET  /v1/quilts/{quilt_id}/patches`            list quilt patches (M5)
//!   - `GET  /status`                                  liveness
//!
//! It holds ONE [`LocalnetMockStore`] (built once at startup, shared via `Arc`), so a
//! blob stored over HTTP is byte-identical and readable both via this API and via the
//! Rust `WalrusLocalClient` SDK mirror (same shared filesystem dir). The suibase-daemon
//! is NOT involved. See docs/dev/SB_LOCAL_PLAN.md.

use std::{net::SocketAddr, str::FromStr, sync::Arc};

use anyhow::{Context as _, Result};
use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{
        header::{self, HeaderName},
        HeaderMap, HeaderValue, Method, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sui_types::base_types::SuiAddress;
use walrus_sui::client::{BlobPersistence, PostStoreAction};

use walrus_local_sdk::localnet::{LocalnetMockStore, StoredBlob};

mod wire;
use wire::{BlobStoreResult, RegisterBlobOp};

/// `X-Content-Type-Options` — not a typed header const in `http`, so name it directly.
const X_CONTENT_TYPE_OPTIONS: HeaderName = HeaderName::from_static("x-content-type-options");
/// Cache-Control value the real Walrus aggregator emits (matched verbatim).
const CACHE_CONTROL_VALUE: &str = "public, max-age=86400, stale-while-revalidate=3600";

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "sb-local",
    about = "Suibase localnet Walrus aggregator+publisher HTTP server (nodeless)"
)]
struct Cli {
    /// Address to bind (its OWN setting; independent of the faucet's). localhost-only
    /// by default — these are throwaway localnet keys.
    #[arg(long, default_value = "localhost")]
    bind: String,
    /// TCP port to listen on (its OWN setting; suibase default 45840, the localnet slot
    /// of the Walrus 458xx range).
    #[arg(long, default_value_t = 45840)]
    port: u16,
    /// Suibase workdir. Only `localnet` is supported (nodeless mock is localnet-only).
    #[arg(long, default_value = "localnet")]
    workdir: String,
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();

    if cli.workdir != "localnet" {
        anyhow::bail!(
            "sb-local supports only the localnet workdir (got '{}'); the nodeless \
             Walrus mock is localnet-only",
            cli.workdir
        );
    }

    // Construct the store ONCE (does wallet/RPC setup); share via Arc across requests.
    let store = Arc::new(
        LocalnetMockStore::open()
            .await
            .context("opening the localnet mock store (run 'localnet regen' with walrus_local_enabled=true first?)")?,
    );

    let app = router(store);

    // SocketAddr parsing needs a numeric IP; map the friendly "localhost" -> 127.0.0.1
    // (mirrors the faucet, which does the same because the host can't take "localhost").
    let bind_ip = if cli.bind == "localhost" {
        "127.0.0.1"
    } else {
        cli.bind.as_str()
    };
    let addr: SocketAddr = format!("{}:{}", bind_ip, cli.port)
        .parse()
        .with_context(|| format!("invalid bind address {}:{}", bind_ip, cli.port))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr} (port already in use?)"))?;

    tracing::info!("sb-local listening on http://{addr} (workdir=localnet)");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("sb-local server error")?;

    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

/// Graceful shutdown on Ctrl-C or SIGTERM (so `localnet stop`'s SIGTERM is clean).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("sb-local received shutdown signal; stopping");
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

fn router(store: Arc<LocalnetMockStore>) -> Router {
    Router::new()
        .route("/status", get(status))
        // Aggregator (reads).
        .route("/v1/blobs/{blob_id}", get(get_blob))
        .route("/v1/blobs/by-object-id/{object_id}", get(get_blob_by_object_id))
        // Publisher (writes).
        .route("/v1/blobs", put(put_blob))
        // Quilt routes are added by wire_quilt::mount() in M5.
        .merge(crate::quilt::router())
        // Localhost dev tool: no body-size cap on PUT (blobs/quilts can be large).
        .layer(DefaultBodyLimit::disable())
        .with_state(store)
}

mod quilt;

// ---------------------------------------------------------------------------
// Handlers — status
// ---------------------------------------------------------------------------

/// Liveness. Plain-text `OK`, matching the real daemon's `/status`.
async fn status() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// ---------------------------------------------------------------------------
// Handlers — aggregator (GET)
// ---------------------------------------------------------------------------

/// `GET /v1/blobs/{blob_id}` — serve the raw bytes with the Walrus aggregator headers
/// (`ETag`, `Cache-Control`, `X-Content-Type-Options: nosniff`), supporting HTTP
/// `Range:` (→ 206). `404` if the blob is absent.
async fn get_blob(
    method: Method,
    headers: HeaderMap,
    State(store): State<Arc<LocalnetMockStore>>,
    Path(blob_id): Path<String>,
) -> Response {
    // Malformed id -> 400 (bad request), like the real aggregator; valid-but-absent -> 404.
    if !store.is_valid_blob_id(&blob_id) {
        return bad_request(&format!("malformed blob id {blob_id}"));
    }
    if !store.has_blob(&blob_id) {
        return not_found("BLOB_NOT_FOUND", &format!("blob {blob_id} not found"));
    }
    match store.read(&blob_id).await {
        Ok(bytes) => serve_bytes(&method, &headers, &blob_id, bytes),
        Err(_) => not_found("BLOB_NOT_FOUND", &format!("blob {blob_id} not found")),
    }
}

/// `GET /v1/blobs/by-object-id/{object_id}` — resolve a Sui `Blob` object id to its
/// content id and serve the bytes (same headers/range semantics as `get_blob`).
async fn get_blob_by_object_id(
    method: Method,
    headers: HeaderMap,
    State(store): State<Arc<LocalnetMockStore>>,
    Path(object_id): Path<String>,
) -> Response {
    match store.read_by_object_id(&object_id).await {
        Ok((blob_id, bytes)) => serve_bytes(&method, &headers, &blob_id, bytes),
        Err(_) => not_found(
            "BLOB_NOT_FOUND",
            &format!("no readable blob for object {object_id}"),
        ),
    }
}

// ---------------------------------------------------------------------------
// Handlers — publisher (PUT)
// ---------------------------------------------------------------------------

/// Publisher query params (a superset of what we act on). Mirrors the real
/// `PublisherQuery`; unknown params are tolerated (no `deny_unknown_fields`) so newer
/// clients stay drop-in. `deletable`/`permanent`/`encoding_type`/`quilt_version`/
/// `reuse_resources`/`force` are accepted but no-ops on the nodeless mock (always
/// Permanent, single RS2 encoding).
#[derive(Debug, Default, Deserialize)]
pub(crate) struct PublisherQuery {
    /// Storage duration in epochs (default 1, matching the real publisher).
    pub epochs: Option<u32>,
    /// Transfer the created `Blob` object to this Sui address after store.
    pub send_object_to: Option<String>,
    /// Wrap the created `Blob` in a shared object after store.
    #[serde(default)]
    pub share: bool,
}

/// `PUT /v1/blobs` — store the request body as a certified `Blob` and return the
/// camelCase `BlobStoreResult` JSON (200), wire-identical to the real publisher.
async fn put_blob(
    State(store): State<Arc<LocalnetMockStore>>,
    Query(q): Query<PublisherQuery>,
    body: Bytes,
) -> Response {
    let epochs = q.epochs.unwrap_or(1);
    // Reject epochs=0 cleanly (a real store would reserve 0 epochs of storage, which the
    // chain rejects). Fast 400 instead of attempting a doomed on-chain reserve.
    if epochs == 0 {
        return bad_request("epochs must be >= 1");
    }
    let post_store = match resolve_post_store(&q) {
        Ok(ps) => ps,
        Err(resp) => return resp,
    };

    // sb-local stores Permanent (the publisher's deprecated `deletable` flag is a no-op,
    // matching the real walrus daemon); the Deletable path is exercised via the Rust mirror.
    match store
        .store_blob(body.as_ref(), epochs, BlobPersistence::Permanent, post_store)
        .await
    {
        Ok(stored) => (StatusCode::OK, Json(blob_store_result(stored))).into_response(),
        Err(e) => internal_error(&format!("store failed: {e:#}")),
    }
}

/// Map `send_object_to` xor `share` to a `PostStoreAction` (default Keep).
pub(crate) fn resolve_post_store(q: &PublisherQuery) -> Result<PostStoreAction, Response> {
    match (q.send_object_to.as_deref(), q.share) {
        (Some(_), true) => Err(bad_request(
            "`send_object_to` and `share` are mutually exclusive",
        )),
        (Some(addr), false) => SuiAddress::from_str(addr)
            .map(PostStoreAction::TransferTo)
            .map_err(|e| bad_request(&format!("invalid send_object_to address: {e}"))),
        (None, true) => Ok(PostStoreAction::Share),
        (None, false) => Ok(PostStoreAction::Keep),
    }
}

/// Build the wire `BlobStoreResult` from a [`StoredBlob`] (shared with the quilt PUT).
pub(crate) fn blob_store_result(s: StoredBlob) -> BlobStoreResult {
    if s.newly_created {
        BlobStoreResult::NewlyCreated {
            blob_object: s.blob,
            resource_operation: RegisterBlobOp::RegisterFromScratch {
                encoded_length: s.encoded_length,
                epochs_ahead: s.epochs,
            },
            // The nodeless mock pays WAL from the faucet-funded exchange; report 0
            // (clients use the blobId/objectId, not the cost, on localnet).
            cost: 0,
            shared_blob_object: s.shared_object_id.map(|id| id.to_string()),
        }
    } else {
        BlobStoreResult::AlreadyCertified {
            blob_id: s.blob.blob_id.to_string(),
            object: s.blob.id.to_string(),
            end_epoch: s.blob.storage.end_epoch,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared response helpers
// ---------------------------------------------------------------------------

/// Serve blob bytes with the Walrus aggregator headers, honoring a single
/// `Range: bytes=start-end` request (→ 206 + `Content-Range`/`Content-Length`).
pub(crate) fn serve_bytes(
    method: &Method,
    req_headers: &HeaderMap,
    etag: &str,
    bytes: Vec<u8>,
) -> Response {
    let total = bytes.len() as u64;

    let mut response = if let Some(range) = req_headers.get(header::RANGE) {
        match parse_single_range(range, total) {
            Ok((start, end)) => {
                let slice = bytes[start as usize..=end as usize].to_vec();
                let mut r =
                    (StatusCode::PARTIAL_CONTENT, Body::from(slice)).into_response();
                let cr = format!("bytes {start}-{end}/{total}");
                if let Ok(v) = HeaderValue::from_str(&cr) {
                    r.headers_mut().insert(header::CONTENT_RANGE, v);
                }
                r.headers_mut()
                    .insert(header::CONTENT_LENGTH, HeaderValue::from(end - start + 1));
                r
            }
            Err(resp) => return resp,
        }
    } else {
        // Use Body::from (NOT Json/Vec<u8>) so we control Content-Type exactly like the
        // real daemon (it does not set a default Content-Type; see create_response_from_blob).
        (StatusCode::OK, Body::from(bytes)).into_response()
    };

    populate_aggregator_headers(method, req_headers, etag, response.headers_mut());
    response
}

/// Insert the aggregator response headers exactly as the real daemon does:
/// `X-Content-Type-Options: nosniff`, the caching `Cache-Control`, `ETag` = blob id,
/// and a mirrored `Content-Type` (Accept without `*`, else the GET request's Content-Type).
fn populate_aggregator_headers(
    method: &Method,
    req: &HeaderMap,
    etag: &str,
    out: &mut HeaderMap,
) {
    out.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    out.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL_VALUE),
    );
    if let Ok(v) = HeaderValue::from_str(etag) {
        out.insert(header::ETAG, v);
    }

    let mirror = if let Some(accept) = req.get(header::ACCEPT) {
        if accept.as_bytes().contains(&b'*') {
            None
        } else {
            Some(accept.clone())
        }
    } else if method == Method::GET {
        req.get(header::CONTENT_TYPE).cloned()
    } else {
        None
    };
    if let Some(ct) = mirror {
        out.insert(header::CONTENT_TYPE, ct);
    }
}

/// Parse a single `Range: bytes=start-end` header against a known `total` length.
/// Returns `(start, end)` inclusive (end clamped to `total-1`), or an error response
/// (`400` malformed, `416` unsatisfiable) — matching the real daemon's single-range,
/// closed-interval support.
fn parse_single_range(value: &HeaderValue, total: u64) -> Result<(u64, u64), Response> {
    let s = value
        .to_str()
        .map_err(|_| range_bad("invalid range header format"))?
        .trim();
    let spec = s
        .strip_prefix("bytes=")
        .ok_or_else(|| range_bad("range must start with `bytes=`"))?;
    if spec.contains(',') {
        return Err(range_bad("only one range per request is supported"));
    }
    let (a, b) = spec
        .split_once('-')
        .ok_or_else(|| range_bad("range must be `start-end`"))?;
    let start: u64 = a
        .trim()
        .parse()
        .map_err(|_| range_bad("must provide a start index"))?;
    let end_str = b.trim();
    if end_str.is_empty() {
        return Err(range_bad("must provide an end index"));
    }
    let end: u64 = end_str
        .parse()
        .map_err(|_| range_bad("invalid end index"))?;
    if start > end {
        return Err(range_bad("start index must be <= end index"));
    }
    if total == 0 || start >= total {
        return Err(range_unsatisfiable(total));
    }
    Ok((start, end.min(total - 1)))
}

fn range_bad(msg: &str) -> Response {
    bad_request(msg)
}

fn range_unsatisfiable(total: u64) -> Response {
    let mut r = (
        StatusCode::RANGE_NOT_SATISFIABLE,
        Json(error_body("INVALID_BYTE_RANGE", "range not satisfiable")),
    )
        .into_response();
    if let Ok(v) = HeaderValue::from_str(&format!("bytes */{total}")) {
        r.headers_mut().insert(header::CONTENT_RANGE, v);
    }
    r
}

pub(crate) fn not_found(reason: &str, message: &str) -> Response {
    (StatusCode::NOT_FOUND, Json(error_body(reason, message))).into_response()
}

pub(crate) fn bad_request(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(error_body("BAD_REQUEST", message)),
    )
        .into_response()
}

pub(crate) fn internal_error(message: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(error_body("INTERNAL", message)),
    )
        .into_response()
}

fn error_body(reason: &str, message: &str) -> serde_json::Value {
    json!({ "error": { "reason": reason, "message": message } })
}
