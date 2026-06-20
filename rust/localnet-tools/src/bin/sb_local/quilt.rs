// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Quilt HTTP routes (M5) for sb-local — drop-in for the real Walrus daemon's quilt
//! API, backed by [`LocalnetMockStore`]'s nodeless quilt engine:
//!   - `PUT  /v1/quilts`                                multipart store -> `QuiltStoreResult`
//!   - `GET  /v1/blobs/by-quilt-patch-id/{id}`          read a patch by its QuiltPatchId
//!   - `GET  /v1/blobs/by-quilt-id/{quilt_id}/{ident}`  read a patch by quilt id + identifier
//!   - `GET  /v1/quilts/{quilt_id}/patches`             list patches -> `[QuiltPatchItem]`
//!
//! A quilt is a single Permanent blob packing many named blobs + an embedded index;
//! construction/reading are 100% client-side pure compute (no storage nodes).

use std::{collections::BTreeMap, sync::Arc};

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use walrus_local_sdk::localnet::{LocalnetMockStore, QuiltInput, QuiltPatchData};

use crate::wire::{QuiltPatchItem, QuiltStoreResult, StoredQuiltPatch};
use crate::{bad_request, blob_store_result, internal_error, not_found, resolve_post_store, serve_bytes, PublisherQuery};

/// Header carrying the patch identifier on a quilt-patch read (matches the real daemon).
const X_QUILT_PATCH_IDENTIFIER: HeaderName = HeaderName::from_static("x-quilt-patch-identifier");
/// Multipart field name carrying the optional JSON `[{identifier, tags}]` metadata.
const METADATA_FIELD: &str = "_metadata";

/// Quilt sub-router, typed to the shared store state so it merges into the main router.
pub fn router() -> Router<Arc<LocalnetMockStore>> {
    Router::new()
        .route("/v1/quilts", put(put_quilt))
        .route(
            "/v1/blobs/by-quilt-patch-id/{quilt_patch_id}",
            get(get_by_patch_id),
        )
        .route(
            "/v1/blobs/by-quilt-id/{quilt_id}/{identifier}",
            get(get_by_quilt_and_identifier),
        )
        .route("/v1/quilts/{quilt_id}/patches", get(list_patches))
}

// ---------------------------------------------------------------------------
// PUT /v1/quilts (multipart)
// ---------------------------------------------------------------------------

/// `PUT /v1/quilts` — pack the uploaded files (each form field-name = patch identifier)
/// into one quilt blob and store it. Optional `_metadata` field is a JSON array of
/// `{identifier, tags}` attaching tags to patches. Returns the camelCase
/// `QuiltStoreResult` (200), wire-identical to the real publisher.
async fn put_quilt(
    State(store): State<Arc<LocalnetMockStore>>,
    Query(q): Query<PublisherQuery>,
    mut multipart: Multipart,
) -> Response {
    let epochs = q.epochs.unwrap_or(1);
    let post_store = match resolve_post_store(&q) {
        Ok(ps) => ps,
        Err(resp) => return resp,
    };

    // Collect file fields (identifier -> bytes) + the optional _metadata field.
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut metadata_json: Option<Vec<u8>> = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => return bad_request(&format!("invalid multipart body: {e}")),
        };
        let name = field.name().unwrap_or("").to_string();
        let data = match field.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => return bad_request(&format!("reading multipart field {name:?}: {e}")),
        };
        if name == METADATA_FIELD {
            metadata_json = Some(data);
        } else if !name.is_empty() {
            files.push((name, data));
        }
    }

    if files.is_empty() {
        return bad_request(
            "a quilt PUT requires at least one file field (the field name is the patch identifier)",
        );
    }

    let tags_by_ident = match metadata_json {
        Some(bytes) => match parse_metadata_tags(&bytes) {
            Ok(m) => m,
            Err(resp) => return resp,
        },
        None => BTreeMap::new(),
    };

    let patches: Vec<QuiltInput> = files
        .into_iter()
        .map(|(identifier, data)| {
            let tags = tags_by_ident.get(&identifier).cloned().unwrap_or_default();
            QuiltInput {
                identifier,
                data,
                tags,
            }
        })
        .collect();

    match store.store_quilt(patches, epochs, post_store).await {
        Ok(sq) => {
            let result = QuiltStoreResult {
                blob_store_result: blob_store_result(sq.stored),
                stored_quilt_blobs: sq
                    .patches
                    .into_iter()
                    .map(|p| StoredQuiltPatch {
                        identifier: p.identifier,
                        quilt_patch_id: p.quilt_patch_id,
                        range: Some((p.start_index as u64, p.end_index as u64)),
                    })
                    .collect(),
            };
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => internal_error(&format!("quilt store failed: {e:#}")),
    }
}

/// Parse the optional `_metadata` JSON `[{identifier, tags}]` into identifier -> tags.
/// `tags` values may be arbitrary JSON; non-string values are stringified.
fn parse_metadata_tags(
    bytes: &[u8],
) -> Result<BTreeMap<String, BTreeMap<String, String>>, Response> {
    #[derive(serde::Deserialize)]
    struct Entry {
        identifier: String,
        #[serde(default)]
        tags: serde_json::Map<String, serde_json::Value>,
    }
    let entries: Vec<Entry> = serde_json::from_slice(bytes).map_err(|e| {
        bad_request(&format!(
            "invalid _metadata JSON (expected [{{\"identifier\":..,\"tags\":..}}]): {e}"
        ))
    })?;
    let mut out = BTreeMap::new();
    for e in entries {
        let tags = e
            .tags
            .into_iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                (k, val)
            })
            .collect();
        out.insert(e.identifier, tags);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Quilt reads (GET)
// ---------------------------------------------------------------------------

/// `GET /v1/blobs/by-quilt-patch-id/{quilt_patch_id}` — serve a patch by its public id.
async fn get_by_patch_id(
    method: Method,
    headers: HeaderMap,
    State(store): State<Arc<LocalnetMockStore>>,
    Path(quilt_patch_id): Path<String>,
) -> Response {
    // Malformed id -> 400 (bad request), like the real aggregator; valid-but-absent -> 404.
    if !store.is_valid_quilt_patch_id(&quilt_patch_id) {
        return bad_request(&format!("malformed quilt patch id {quilt_patch_id}"));
    }
    match store.read_quilt_patch(&quilt_patch_id).await {
        Ok(patch) => quilt_patch_response(&method, &headers, &quilt_patch_id, patch),
        Err(_) => not_found(
            "QUILT_PATCH_NOT_FOUND",
            &format!("quilt patch {quilt_patch_id} not found"),
        ),
    }
}

/// `GET /v1/blobs/by-quilt-id/{quilt_id}/{identifier}` — serve a patch by quilt id +
/// identifier.
async fn get_by_quilt_and_identifier(
    method: Method,
    headers: HeaderMap,
    State(store): State<Arc<LocalnetMockStore>>,
    Path((quilt_id, identifier)): Path<(String, String)>,
) -> Response {
    match store.read_quilt_blob(&quilt_id, &identifier).await {
        Ok(patch) => quilt_patch_response(&method, &headers, &quilt_id, patch),
        Err(_) => not_found(
            "QUILT_PATCH_NOT_FOUND",
            &format!("patch {identifier:?} not found in quilt {quilt_id}"),
        ),
    }
}

/// `GET /v1/quilts/{quilt_id}/patches` — list the patches in a quilt.
async fn list_patches(
    State(store): State<Arc<LocalnetMockStore>>,
    Path(quilt_id): Path<String>,
) -> Response {
    match store.list_quilt_patches(&quilt_id).await {
        Ok(patches) => {
            let items: Vec<QuiltPatchItem> = patches
                .into_iter()
                .map(|p| QuiltPatchItem {
                    identifier: p.identifier,
                    patch_id: p.quilt_patch_id,
                    tags: p.tags,
                })
                .collect();
            (StatusCode::OK, Json(items)).into_response()
        }
        Err(_) => not_found("QUILT_NOT_FOUND", &format!("quilt {quilt_id} not found")),
    }
}

/// Serve patch bytes with the aggregator headers + the `X-Quilt-Patch-Identifier` header.
fn quilt_patch_response(
    method: &Method,
    headers: &HeaderMap,
    etag: &str,
    patch: QuiltPatchData,
) -> Response {
    let identifier = patch.identifier.clone();
    let mut resp = serve_bytes(method, headers, etag, patch.data);
    if let Ok(v) = HeaderValue::from_str(&identifier) {
        resp.headers_mut().insert(X_QUILT_PATCH_IDENTIFIER, v);
    }
    resp
}
