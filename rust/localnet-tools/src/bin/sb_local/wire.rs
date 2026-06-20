// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Wire types for the Walrus publisher HTTP API, reproduced field-for-field so an
//! existing Walrus agg/pub client parses sb-local's responses unchanged.
//!
//! These mirror `walrus_sdk::node_client::responses::BlobStoreResult` +
//! `walrus_sdk::node_client::resource::RegisterBlobOp` (camelCase, externally-tagged
//! enum). We hand-roll them (rather than depend on the heavy `walrus-sdk` crate) but
//! EMBED the real `walrus_sui::types::move_structs::Blob` for `blobObject`, so the
//! nested object serializes exactly like the real daemon. Cross-checked against the
//! pinned walrus rev (responses.rs / resource.rs).

use std::collections::BTreeMap;

use serde::Serialize;
use walrus_sui::types::move_structs::Blob;

/// Publisher store result (`PUT /v1/blobs` body). Externally tagged + camelCase, so it
/// serializes as `{"newlyCreated": {...}}` / `{"alreadyCertified": {...}}`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BlobStoreResult {
    /// A new certified `Blob` was minted.
    NewlyCreated {
        /// The on-chain `Blob` move struct (serializes camelCase: `id`, `blobId`,
        /// `registeredEpoch`, `certifiedEpoch`, `size`, `storage`, `deletable`, ...).
        blob_object: Blob,
        /// What was done to register the blob (`registerFromScratch` on the mock).
        resource_operation: RegisterBlobOp,
        /// WAL cost (0 on the faucet-funded nodeless localnet).
        cost: u64,
        /// For a `share=true` PUT, the created `SharedBlob` object id.
        #[serde(skip_serializing_if = "Option::is_none")]
        shared_blob_object: Option<String>,
    },
    /// The blob was already certified + unexpired (content dedup). Mirrors the real
    /// `alreadyCertified { blobId, <event|object>, endEpoch }` — the nodeless mock
    /// always knows the object id, so it emits the flattened `object` key.
    AlreadyCertified {
        /// Canonical Walrus blob id (URL-safe base64 string).
        blob_id: String,
        /// The existing `Blob` object id (`0x` + hex) — the flattened `object` arm of
        /// the real `EventOrObjectId`.
        object: String,
        /// Epoch after which the blob's storage expires.
        end_epoch: u32,
    },
}

/// The register operation performed (`resourceOperation`). The nodeless mock always
/// registers from scratch; the other real variants (reuse/extend) never occur here.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RegisterBlobOp {
    /// Storage + blob resources purchased from scratch.
    RegisterFromScratch {
        /// Encoded (erasure-coded) length in bytes.
        encoded_length: u64,
        /// Epochs ahead for which the blob is registered.
        epochs_ahead: u32,
    },
}

/// Result of `PUT /v1/quilts` — mirrors `walrus_sdk::...::QuiltStoreResult`:
/// `{ blobStoreResult, storedQuiltBlobs }`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltStoreResult {
    /// The result of storing the packed quilt as a single blob.
    pub blob_store_result: BlobStoreResult,
    /// Per-input-patch ids + ranges.
    pub stored_quilt_blobs: Vec<StoredQuiltPatch>,
}

/// One element of `storedQuiltBlobs`: `{ identifier, quiltPatchId, range? }`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredQuiltPatch {
    /// The patch identifier (the multipart file field name).
    pub identifier: String,
    /// The public `QuiltPatchId` (URL-safe base64 string).
    pub quilt_patch_id: String,
    /// The patch's sliver range `[start, end]` (omitted if absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<(u64, u64)>,
}

/// One element of `GET /v1/quilts/{quilt_id}/patches` — mirrors the real
/// `QuiltPatchItem`: `{ identifier, patchId, tags }`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltPatchItem {
    /// The patch identifier.
    pub identifier: String,
    /// The public `QuiltPatchId` (URL-safe base64 string).
    pub patch_id: String,
    /// The patch's tags.
    pub tags: BTreeMap<String, String>,
}
