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
        quilt_encoding::{QuiltStoreBlob, QuiltV1},
        EncodingAxis,
    },
    BlobId, EncodingType, QuiltPatchId,
};
use walrus_sdk::{
    error::{ClientError, ClientResult},
    node_client::{
        client_types::StoredQuiltPatch,
        resource::RegisterBlobOp,
        responses::{BlobStoreResult, EventOrObjectId, QuiltStoreResult},
        store_args::StoreArgs,
    },
};
use walrus_sui::types::move_structs::BlobWithAttribute;

use localnet::{LocalnetMockStore, StoredBlob, StoredQuilt};

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
        self.engine.read(&blob_id.to_string()).await.map_err(cerr)
    }

    /// Non-generic convenience read (no turbofish). Equivalent to
    /// `read_blob::<walrus_core::encoding::Primary>`.
    pub async fn read_blob_primary(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>> {
        self.engine.read(&blob_id.to_string()).await.map_err(cerr)
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
    /// Pack blobs into a `QuiltV1` (pure compute). Mirrors
    /// `QuiltClient::construct_quilt::<QuiltVersionV1>`; `encoding_type` is accepted
    /// for signature parity (localnet uses RS2).
    pub async fn construct_quilt(
        &self,
        blobs: &[QuiltStoreBlob<'_>],
        _encoding_type: EncodingType,
    ) -> ClientResult<QuiltV1> {
        self.engine.construct_quilt_v1(blobs).await.map_err(cerr)
    }

    /// Store a constructed quilt. Mirrors
    /// `QuiltClient::reserve_and_store_quilt::<QuiltVersionV1>`.
    pub async fn reserve_and_store_quilt(
        &self,
        quilt: QuiltV1,
        store_args: &StoreArgs,
    ) -> ClientResult<QuiltStoreResult> {
        let sq: StoredQuilt = self
            .engine
            .store_quilt_v1(quilt, store_args.epochs_ahead, store_args.post_store)
            .await
            .map_err(cerr)?;
        Ok(stored_quilt_to_result(sq))
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

fn stored_quilt_to_result(sq: StoredQuilt) -> QuiltStoreResult {
    let stored_quilt_blobs = sq
        .patches
        .into_iter()
        .map(|p| StoredQuiltPatch {
            identifier: p.identifier,
            quilt_patch_id: p.quilt_patch_id,
            range: Some((p.start_index as u64, p.end_index as u64)),
        })
        .collect();
    QuiltStoreResult {
        blob_store_result: stored_blob_to_result(sq.stored),
        stored_quilt_blobs,
    }
}

fn patch_to_quilt_blob(
    patch: localnet::QuiltPatchData,
) -> ClientResult<QuiltStoreBlob<'static>> {
    QuiltStoreBlob::new_owned(patch.data, patch.identifier)
        .map(|b| b.with_tags(patch.tags))
        .map_err(|e| cerr(anyhow::anyhow!("rebuilding quilt patch blob: {e}")))
}
