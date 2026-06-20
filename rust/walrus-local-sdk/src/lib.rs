// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Localnet-only, drop-in mirror of the Mysten Labs Walrus SDK (`walrus_sdk`).
//!
//! [`WalrusLocalClient`] mirrors the method signatures of
//! `walrus_sdk::node_client::WalrusNodeClient<SuiContractClient>` and returns the
//! SDK's **own** types ([`BlobStoreResult`], [`QuiltStoreResult`], [`ClientResult`],
//! …). Caller code is therefore drop-in across networks:
//!
//! ```no_run
//! # async fn f() -> walrus_sdk::error::ClientResult<()> {
//! use walrus_local_sdk::WalrusLocalClient;
//! use walrus_sdk::node_client::store_args::StoreArgs;
//! use walrus_core::encoding::Primary;
//!
//! let client = WalrusLocalClient::for_workdir("localnet").await?;       // nodeless mock
//! let args = StoreArgs::default_with_epochs(5);
//! let results = client.reserve_and_store_blobs(vec![b"hello".to_vec()], &args).await?;
//! let blob_id = results[0].blob_id().unwrap();
//! let bytes = client.read_blob::<Primary>(&blob_id).await?;
//! # Ok(()) }
//! ```
//!
//! The same call sequence runs verbatim against a real `walrus_sdk::WalrusNodeClient`
//! on testnet/mainnet — the caller dispatches by network (see [`compat::WalrusApi`]
//! for a generic seam used by the parity tests).
//!
//! DESIGN INTENT (do not regress): on a real network you use `walrus_sdk` DIRECTLY —
//! this crate inserts no glue there, so a bug here only ever affects localnet (devs).
//! The localnet burden lives in [`localnet`] (the nodeless mock engine) and the thin
//! reshaping below; the real-facing seam ([`compat::WalrusApi`] for `WalrusNodeClient`)
//! is pure forwarding.

pub mod compat;
pub mod localnet;

use sui_types::base_types::ObjectID;
use walrus_core::{
    encoding::{
        quilt_encoding::{
            QuiltApi, QuiltConfigApi, QuiltEncoderApi, QuiltIndexApi, QuiltPatchApi,
            QuiltPatchInternalIdApi, QuiltStoreBlob, QuiltVersion,
        },
        EncodingAxis, EncodingConfig,
    },
    BlobId, EncodingType, QuiltPatchId,
};
use std::num::NonZeroUsize;

use walrus_sdk::{
    error::{ClientError, ClientErrorKind, ClientResult},
    node_client::{
        byte_range_read_client::ReadByteRangeResult,
        client_types::StoredQuiltPatch,
        resource::RegisterBlobOp,
        responses::{BlobStoreResult, EventOrObjectId, QuiltStoreResult},
        store_args::StoreArgs,
    },
};
use walrus_sui::types::move_structs::BlobWithAttribute;

use localnet::{LocalnetMockStore, StoredBlob};

// ---------------------------------------------------------------------------
// Lower-level (non-SDK) handle/metadata types — used by the [`localnet`] engine
// (kept for the HTTP facade `sb-local` + the pool tests). The SDK-mirror surface
// below uses the SDK's own types instead.
// ---------------------------------------------------------------------------

/// Handle to a stored blob: the content id (`blob_id`) and the on-chain Sui object id.
#[derive(Debug, Clone)]
pub struct BlobHandle {
    /// Content id — the canonical Walrus `BlobId` string (URL-safe base64, no padding).
    pub blob_id: String,
    /// The Sui `Blob` object id (`0x` + 64 hex).
    pub object_id: String,
}

/// Metadata about a stored blob.
#[derive(Debug, Clone)]
pub struct BlobMeta {
    pub blob_id: String,
    pub object_id: String,
    pub size: u64,
    /// `Some(epoch)` once certified, else `None`.
    pub certified_epoch: Option<u32>,
    /// Epoch after which the storage expires.
    pub end_epoch: u32,
}

/// Handle to a storage pool: a pre-reserved chunk of (encoded) storage capacity
/// that many pooled blobs share for the pool's lifetime.
#[derive(Debug, Clone)]
pub struct PoolHandle {
    /// The Sui `StoragePool` object id (`0x` + hex).
    pub pool_id: String,
}

/// Live state of a storage pool. Capacities are in **encoded** bytes (the unit the
/// pool reserves), matching Walrus on-chain accounting.
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub pool_id: String,
    pub start_epoch: u32,
    pub end_epoch: u32,
    pub reserved_capacity_bytes: u64,
    pub used_bytes: u64,
    pub blob_count: u64,
}

// ---------------------------------------------------------------------------
// Error bridge: the mirror returns the SDK's own ClientResult. The engine returns
// anyhow::Error, which is NOT itself `std::error::Error`, so we wrap it in a tiny
// newtype that is, then hand it to `ClientError::other` (the SDK's escape hatch for
// arbitrary errors). Localnet failures thus surface as `ClientErrorKind::Other`.
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct LocalError(String);

impl std::fmt::Display for LocalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LocalError {}

/// Convert an engine `anyhow::Error` into the SDK's `ClientError`.
fn cerr(e: anyhow::Error) -> ClientError {
    ClientError::other(LocalError(format!("{e:#}")))
}

/// Reshape the engine's rich [`StoredBlob`] into the SDK's wire `BlobStoreResult`.
fn stored_blob_to_result(s: StoredBlob) -> BlobStoreResult {
    if s.newly_created {
        BlobStoreResult::NewlyCreated {
            blob_object: s.blob,
            resource_operation: RegisterBlobOp::RegisterFromScratch {
                encoded_length: s.encoded_length,
                epochs_ahead: s.epochs,
            },
            // Localnet does not meter WAL cost (the exchange mints it 1:1 on demand).
            cost: 0,
            shared_blob_object: s.shared_object_id,
        }
    } else {
        // Deduped to an existing certified, unexpired Blob.
        BlobStoreResult::AlreadyCertified {
            blob_id: s.blob.blob_id,
            event_or_object: EventOrObjectId::Object(s.blob.id),
            end_epoch: s.blob.storage.end_epoch,
        }
    }
}

// ---------------------------------------------------------------------------
// WalrusLocalClient — the SDK-mirroring façade.
// ---------------------------------------------------------------------------

/// Drop-in localnet mirror of `walrus_sdk::node_client::WalrusNodeClient<SuiContractClient>`.
///
/// Construct with [`WalrusLocalClient::for_workdir`] (only `"localnet"` is valid —
/// for a real network you use `walrus_sdk` directly). Wraps the nodeless mock
/// [`LocalnetMockStore`] and reshapes its results into the SDK's own types.
pub struct WalrusLocalClient {
    engine: LocalnetMockStore,
}

impl WalrusLocalClient {
    /// Open the localnet client (reads the deploy-written descriptor + the workdir's
    /// `client.yaml`). Equivalent to `for_workdir("localnet")`.
    pub async fn open() -> ClientResult<Self> {
        Ok(Self {
            engine: LocalnetMockStore::open().await.map_err(cerr)?,
        })
    }

    /// Construct for the named suibase workdir. Only `"localnet"` is supported; any
    /// other name is an error directing the caller to use `walrus_sdk` directly.
    pub async fn for_workdir(name: &str) -> ClientResult<Self> {
        if name != "localnet" {
            return Err(cerr(anyhow::anyhow!(
                "walrus-local-sdk is localnet-only; for workdir '{name}' use walrus_sdk directly"
            )));
        }
        Self::open().await
    }

    /// Access the underlying nodeless engine — the localnet-only lower-level API
    /// (rich `StoredBlob`, storage pools, quilt index). NOT part of the SDK mirror;
    /// used by the `sb-local` HTTP facade and the pool tests.
    pub fn engine(&self) -> &LocalnetMockStore {
        &self.engine
    }

    // ----- store (mirrors WalrusNodeClient::reserve_and_store_blobs) --------

    /// Store a list of blobs, returning one `BlobStoreResult` per input (in order).
    /// Mirrors `StoreBlobsApi::reserve_and_store_blobs`. Honors `store_args.epochs_ahead`
    /// and `store_args.post_store`; the localnet engine always certifies (held-key,
    /// N=1 committee) and serves bytes from the filesystem.
    pub async fn reserve_and_store_blobs(
        &self,
        blobs: Vec<Vec<u8>>,
        store_args: &StoreArgs,
    ) -> ClientResult<Vec<BlobStoreResult>> {
        let mut out = Vec::with_capacity(blobs.len());
        for bytes in blobs {
            let stored = self
                .engine
                .store_blob(&bytes, store_args.epochs_ahead, store_args.post_store)
                .await
                .map_err(cerr)?;
            out.push(stored_blob_to_result(stored));
        }
        Ok(out)
    }

    // ----- read (mirrors WalrusNodeClient::read_blob) ----------------------

    /// Read a blob's bytes by id. Mirrors `WalrusNodeClient::read_blob::<U>` — the
    /// encoding axis `U` is irrelevant on localnet (bytes are served whole from the
    /// filesystem, not reconstructed from slivers), so it is accepted and ignored.
    pub async fn read_blob<U: EncodingAxis>(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>> {
        self.read_bytes(blob_id).await
    }

    /// Non-generic convenience read (no turbofish). Equivalent to
    /// `read_blob::<walrus_core::encoding::Primary>`.
    pub async fn read_blob_primary(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>> {
        self.read_bytes(blob_id).await
    }

    /// Shared read path. A missing blob surfaces the SDK's `BlobIdDoesNotExist` kind
    /// (matching `walrus_sdk`'s `read_blob`, which returns `ClientErrorKind::BlobIdDoesNotExist`
    /// for an unknown id) rather than a generic `Other` — so error-matching callers are drop-in.
    async fn read_bytes(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>> {
        let id = blob_id.to_string();
        if !self.engine.has_blob(&id) {
            return Err(ClientError::from(ClientErrorKind::BlobIdDoesNotExist));
        }
        self.engine.read(&id).await.map_err(cerr)
    }

    // ----- delete (mirrors WalrusNodeClient::delete_owned_blob) -------------

    /// Delete owned blob(s) with this id, returning the number removed. Mirrors
    /// `WalrusNodeClient::delete_owned_blob`. Idempotent: deleting an absent blob
    /// returns `0`.
    pub async fn delete_owned_blob(&self, blob_id: &BlobId) -> ClientResult<usize> {
        let id = blob_id.to_string();
        let existed = self.engine.has_blob(&id);
        self.engine.delete(&id).await.map_err(cerr)?;
        Ok(if existed { 1 } else { 0 })
    }

    // ----- status / object lookup ------------------------------------------

    /// Fetch the on-chain `Blob` (+ optional attribute) for an object id. Mirrors
    /// `WalrusNodeClient::get_blob_by_object_id`.
    pub async fn get_blob_by_object_id(
        &self,
        blob_object_id: &ObjectID,
    ) -> ClientResult<BlobWithAttribute> {
        self.engine
            .get_blob_by_object_id(blob_object_id)
            .await
            .map_err(cerr)
    }

    // ----- quilt sub-client ------------------------------------------------

    /// A quilt sub-client. Mirrors `WalrusNodeClient::quilt_client`.
    pub fn quilt_client(&self) -> LocalQuiltClient<'_> {
        LocalQuiltClient { engine: &self.engine }
    }

    /// A byte-range read sub-client. Mirrors `WalrusNodeClient::byte_range_read_client`.
    pub fn byte_range_read_client(&self) -> LocalByteRangeReadClient<'_> {
        LocalByteRangeReadClient { engine: &self.engine }
    }
}

// ---------------------------------------------------------------------------
// LocalQuiltClient — mirrors walrus_sdk's QuiltClient (V1-specialized).
// ---------------------------------------------------------------------------

/// Localnet quilt sub-client. Mirrors `walrus_sdk::node_client::quilt_client::QuiltClient`,
/// **specialized to `QuiltVersionV1`** (the only quilt version this walrus rev defines).
/// The SDK methods are generic over `V: QuiltVersion`; here they are concrete `QuiltV1`.
pub struct LocalQuiltClient<'a> {
    engine: &'a LocalnetMockStore,
}

impl LocalQuiltClient<'_> {
    /// Pack blobs into a quilt (pure compute). Generic over `V: QuiltVersion`, mirroring
    /// `walrus_sdk`'s `QuiltClient::construct_quilt::<V>` — it dispatches through `V`'s
    /// associated `QuiltConfig` encoder, exactly like the SDK. The only difference: the
    /// shard count comes straight from the localnet engine instead of the SDK's committee.
    pub async fn construct_quilt<V: QuiltVersion>(
        &self,
        blobs: &[QuiltStoreBlob<'_>],
        encoding_type: EncodingType,
    ) -> ClientResult<V::Quilt> {
        let n_shards = self.engine.n_shards().await.map_err(cerr)?;
        let encoder =
            V::QuiltConfig::get_encoder(EncodingConfig::new(n_shards).get_for_type(encoding_type), blobs);
        encoder.construct_quilt().map_err(ClientError::from)
    }

    /// Store a constructed quilt. Generic over `V: QuiltVersion`, mirroring `walrus_sdk`'s
    /// `QuiltClient::reserve_and_store_quilt::<V>` body verbatim — snapshot the index,
    /// store the packed bytes as one blob, then map each patch to a `StoredQuiltPatch`.
    /// The only difference: the packed bytes go through the localnet engine's `store_blob`
    /// (held-key certify, fs bytes) instead of the SDK's node store.
    pub async fn reserve_and_store_quilt<V: QuiltVersion>(
        &self,
        quilt: V::Quilt,
        store_args: &StoreArgs,
    ) -> ClientResult<QuiltStoreResult> {
        let quilt_index = quilt.quilt_index().map_err(ClientError::from)?.clone();
        let stored = self
            .engine
            .store_blob(&quilt.into_data(), store_args.epochs_ahead, store_args.post_store)
            .await
            .map_err(cerr)?;
        let blob_store_result = stored_blob_to_result(stored);

        if blob_store_result.is_not_stored() {
            return Ok(QuiltStoreResult {
                blob_store_result,
                stored_quilt_blobs: Vec::new(),
            });
        }

        let blob_id = blob_store_result
            .blob_id()
            .expect("a stored quilt blob has an id");
        let stored_quilt_blobs = quilt_index
            .patches()
            .iter()
            .map(|patch| {
                let sliver_indices = patch.quilt_patch_internal_id().sliver_indices();
                let start_index = sliver_indices.first().map(|s| s.get()).unwrap_or(0);
                let end_index = sliver_indices
                    .last()
                    .map(|s| s.get())
                    .map(|s| s + 1)
                    .unwrap_or(0);
                StoredQuiltPatch::new(blob_id, patch.identifier(), patch.quilt_patch_internal_id())
                    .with_range(start_index.into(), end_index.into())
            })
            .collect::<Vec<_>>();

        Ok(QuiltStoreResult {
            blob_store_result,
            stored_quilt_blobs,
        })
    }

    /// Read patches by identifier. Mirrors `QuiltClient::get_blobs_by_identifiers`.
    pub async fn get_blobs_by_identifiers(
        &self,
        quilt_id: &BlobId,
        identifiers: &[&str],
    ) -> ClientResult<Vec<QuiltStoreBlob<'static>>> {
        let quilt_id = quilt_id.to_string();
        let mut out = Vec::with_capacity(identifiers.len());
        for ident in identifiers {
            let patch = self
                .engine
                .read_quilt_blob(&quilt_id, ident)
                .await
                .map_err(cerr)?;
            out.push(patch_to_quilt_blob(patch)?);
        }
        Ok(out)
    }

    /// Read patches by their public `QuiltPatchId`. Mirrors `QuiltClient::get_blobs_by_ids`.
    pub async fn get_blobs_by_ids(
        &self,
        quilt_patch_ids: &[QuiltPatchId],
    ) -> ClientResult<Vec<QuiltStoreBlob<'static>>> {
        let mut out = Vec::with_capacity(quilt_patch_ids.len());
        for qpid in quilt_patch_ids {
            let patch = self
                .engine
                .read_quilt_patch(&qpid.to_string())
                .await
                .map_err(cerr)?;
            out.push(patch_to_quilt_blob(patch)?);
        }
        Ok(out)
    }

    /// Read patches whose tags contain `target_tag == target_value`. Mirrors
    /// `QuiltClient::get_blobs_by_tag` (the localnet engine retains the per-patch tags
    /// in the quilt index).
    pub async fn get_blobs_by_tag(
        &self,
        quilt_id: &BlobId,
        target_tag: &str,
        target_value: &str,
    ) -> ClientResult<Vec<QuiltStoreBlob<'static>>> {
        let quilt_id = quilt_id.to_string();
        let patches = self
            .engine
            .list_quilt_patches(&quilt_id)
            .await
            .map_err(cerr)?;
        let mut out = Vec::new();
        for p in patches {
            if p.tags.get(target_tag).map(String::as_str) == Some(target_value) {
                let patch = self
                    .engine
                    .read_quilt_blob(&quilt_id, &p.identifier)
                    .await
                    .map_err(cerr)?;
                out.push(patch_to_quilt_blob(patch)?);
            }
        }
        Ok(out)
    }

    /// Read every patch in a quilt. Mirrors `QuiltClient::get_all_blobs`.
    pub async fn get_all_blobs(
        &self,
        quilt_id: &BlobId,
    ) -> ClientResult<Vec<QuiltStoreBlob<'static>>> {
        let quilt_id = quilt_id.to_string();
        let patches = self
            .engine
            .list_quilt_patches(&quilt_id)
            .await
            .map_err(cerr)?;
        let mut out = Vec::with_capacity(patches.len());
        for p in patches {
            let patch = self
                .engine
                .read_quilt_blob(&quilt_id, &p.identifier)
                .await
                .map_err(cerr)?;
            out.push(patch_to_quilt_blob(patch)?);
        }
        Ok(out)
    }
}

fn patch_to_quilt_blob(
    patch: localnet::QuiltPatchData,
) -> ClientResult<QuiltStoreBlob<'static>> {
    QuiltStoreBlob::new_owned(patch.data, patch.identifier)
        .map(|b| b.with_tags(patch.tags))
        .map_err(|e| cerr(anyhow::anyhow!("rebuilding quilt patch blob: {e}")))
}

// ---------------------------------------------------------------------------
// LocalByteRangeReadClient — mirrors walrus_sdk's ByteRangeReadClient.
// ---------------------------------------------------------------------------

/// Build the EXACT `ByteRangeReadInputError` walrus_sdk raises for invalid range inputs.
/// Because `ClientErrorKind` is a public, constructible enum, the mirror returns the same
/// error kind + message as the SDK — kind-for-kind drop-in parity.
fn byte_range_input_err(msg: impl Into<String>) -> ClientError {
    ClientError::from(ClientErrorKind::ByteRangeReadInputError(msg.into()))
}

/// Validate the `(start, length)` inputs the way walrus_sdk's `read_byte_range` does,
/// BEFORE touching the blob (the SDK validates inputs before fetching status/metadata).
/// Pure — unit-tested exhaustively.
fn validate_byte_range_inputs(
    start_byte_position: u64,
    byte_length: u64,
) -> ClientResult<(usize, NonZeroUsize)> {
    let start = usize::try_from(start_byte_position)
        .map_err(|_| byte_range_input_err("start byte position is too large to convert to usize"))?;
    let length = NonZeroUsize::new(
        usize::try_from(byte_length)
            .map_err(|_| byte_range_input_err("byte length is too large to convert to usize"))?,
    )
    .ok_or_else(|| byte_range_input_err("byte length cannot be zero"))?;
    Ok((start, length))
}

/// Bounds-check the (validated) range against `bytes` and slice it out, matching
/// walrus_sdk's `calculate_and_validate_end_byte_position` (same overflow / out-of-bounds
/// error kinds + messages). On localnet the blob bytes are whole on disk (no slivers), so
/// the range is a plain slice. Pure — unit-tested exhaustively.
fn finish_byte_range(
    bytes: &[u8],
    start: usize,
    length: NonZeroUsize,
) -> ClientResult<ReadByteRangeResult> {
    let blob_size = bytes.len();
    let end = start
        .checked_add(length.get())
        .ok_or_else(|| byte_range_input_err("byte range overflow"))?;
    if end > blob_size {
        return Err(byte_range_input_err(format!(
            "byte range out of bounds: requested {start}-{end}, blob size is {blob_size}"
        )));
    }
    Ok(ReadByteRangeResult {
        data: bytes[start..end].to_vec(),
        unencoded_blob_size: blob_size as u64,
    })
}

/// Full pure validate + slice (the order is: input validation, then bounds/slice). Lets
/// unit tests exercise the whole `read_byte_range` contract without a live localnet.
#[cfg(test)]
fn slice_byte_range(
    bytes: &[u8],
    start_byte_position: u64,
    byte_length: u64,
) -> ClientResult<ReadByteRangeResult> {
    let (start, length) = validate_byte_range_inputs(start_byte_position, byte_length)?;
    finish_byte_range(bytes, start, length)
}

/// Localnet byte-range read sub-client. Mirrors
/// `walrus_sdk::node_client::byte_range_read_client::ByteRangeReadClient`. Critical for
/// clients that fetch slices of large blobs (range requests) without pulling the whole blob.
pub struct LocalByteRangeReadClient<'a> {
    engine: &'a LocalnetMockStore,
}

impl LocalByteRangeReadClient<'_> {
    /// Read `[start_byte_position, start_byte_position + byte_length)` from a blob.
    /// Mirrors `ByteRangeReadClient::read_byte_range` — same signature, same
    /// `ReadByteRangeResult`, same input-validation error kinds. Input validation runs
    /// before the blob is touched (SDK order); then the bytes are read and sliced.
    pub async fn read_byte_range(
        &self,
        blob_id: &BlobId,
        start_byte_position: u64,
        byte_length: u64,
    ) -> ClientResult<ReadByteRangeResult> {
        let (start, length) = validate_byte_range_inputs(start_byte_position, byte_length)?;
        let bytes = self.engine.read(&blob_id.to_string()).await.map_err(cerr)?;
        finish_byte_range(&bytes, start, length)
    }
}

#[cfg(test)]
mod byte_range_tests {
    use super::{slice_byte_range, validate_byte_range_inputs};
    use walrus_sdk::error::ClientErrorKind;

    fn input_err_msg(e: walrus_sdk::error::ClientError) -> String {
        match e.kind() {
            ClientErrorKind::ByteRangeReadInputError(m) => m.clone(),
            other => panic!("expected ByteRangeReadInputError, got {other:?}"),
        }
    }

    #[test]
    fn ok_ranges_match_the_plain_slice() {
        let bytes: Vec<u8> = (0u8..=255).cycle().take(10_000).collect();
        let cases = [
            (0u64, bytes.len() as u64), // whole blob
            (0, 1),                     // first byte
            (0, 100),                   // prefix
            (9_900, 100),               // suffix (ends exactly at len)
            (9_999, 1),                 // last byte
            (1234, 1),                  // single middle byte
            (1234, 4321),               // middle span
            (4096, 4096),               // sliver-sized middle span
            (5000, 5000),               // second half
        ];
        for (start, len) in cases {
            let r = slice_byte_range(&bytes, start, len)
                .unwrap_or_else(|e| panic!("range {start}+{len} should be Ok: {e:?}"));
            let s = start as usize;
            assert_eq!(r.data, &bytes[s..s + len as usize], "data for {start}+{len}");
            assert_eq!(r.unencoded_blob_size, bytes.len() as u64);
        }
    }

    #[test]
    fn zero_length_is_an_input_error() {
        // Validated BEFORE any blob access (matches the SDK), so it does not depend on bytes.
        let e = validate_byte_range_inputs(0, 0).unwrap_err();
        assert_eq!(input_err_msg(e), "byte length cannot be zero");
    }

    #[test]
    fn out_of_bounds_is_rejected() {
        let bytes = vec![7u8; 100];
        // start past end
        let e = slice_byte_range(&bytes, 100, 1).unwrap_err();
        assert_eq!(input_err_msg(e), "byte range out of bounds: requested 100-101, blob size is 100");
        // length past end
        let e = slice_byte_range(&bytes, 0, 101).unwrap_err();
        assert_eq!(input_err_msg(e), "byte range out of bounds: requested 0-101, blob size is 100");
        // last byte + 1
        let e = slice_byte_range(&bytes, 99, 2).unwrap_err();
        assert_eq!(input_err_msg(e), "byte range out of bounds: requested 99-101, blob size is 100");
    }

    #[test]
    fn exact_end_is_allowed() {
        let bytes = vec![7u8; 100];
        let r = slice_byte_range(&bytes, 99, 1).unwrap();
        assert_eq!(r.data, vec![7u8]);
        let r = slice_byte_range(&bytes, 0, 100).unwrap();
        assert_eq!(r.data.len(), 100);
    }

    #[test]
    fn overflow_is_an_input_error() {
        let bytes = vec![0u8; 16];
        // start + length overflows usize/u64 addition.
        let e = slice_byte_range(&bytes, u64::MAX, 1).unwrap_err();
        assert_eq!(input_err_msg(e), "byte range overflow");
    }

    #[test]
    fn empty_blob_rejects_any_range() {
        let bytes: Vec<u8> = vec![];
        let e = slice_byte_range(&bytes, 0, 1).unwrap_err();
        assert_eq!(input_err_msg(e), "byte range out of bounds: requested 0-1, blob size is 0");
    }
}

#[cfg(test)]
mod client_tests {
    use super::WalrusLocalClient;

    #[tokio::test]
    async fn for_workdir_rejects_non_localnet() {
        // The guard rejects any non-localnet name BEFORE opening anything, so this is a
        // pure check that needs no live localnet (runs in the always-on `cargo test --lib`).
        assert!(WalrusLocalClient::for_workdir("testnet").await.is_err());
        assert!(WalrusLocalClient::for_workdir("mainnet").await.is_err());
        assert!(WalrusLocalClient::for_workdir("bogus").await.is_err());
    }
}
