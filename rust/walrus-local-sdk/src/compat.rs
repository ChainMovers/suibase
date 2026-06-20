// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! [`WalrusApi`] — a minimal generic seam over the parts of the Walrus client surface
//! that are byte-for-byte identical on a real network and on localnet. It lets a caller
//! write ONE generic function and run it against either backend (drop-in dispatch and,
//! crucially, the parity tests that prove the mirror stays signature-compatible).
//!
//! Only the **non-generic blob core** lives in the trait. Generic reads
//! (`read_blob::<U>`), the quilt sub-client, and introspection stay as inherent
//! methods on each client — their type generics / borrowed sub-client types resist a
//! single object-safe trait, and forcing them in would add glue (= risk) for little gain.
//!
//! The real impl is **pure forwarding** to `walrus_sdk` (exactly one SDK call per
//! method, no logic): the whole crate's design keeps the real path transparent and
//! pushes the bug burden onto localnet (dev-only).

use walrus_core::BlobId;
use walrus_sdk::{
    error::ClientResult,
    node_client::{responses::BlobStoreResult, store_args::StoreArgs},
};

/// The common blob-core surface shared by [`crate::WalrusLocalClient`] (localnet) and
/// `walrus_sdk::node_client::WalrusNodeClient<SuiContractClient>` (real networks).
#[allow(async_fn_in_trait)]
pub trait WalrusApi {
    /// See `walrus_sdk::node_client::StoreBlobsApi::reserve_and_store_blobs`.
    async fn reserve_and_store_blobs(
        &self,
        blobs: Vec<Vec<u8>>,
        store_args: &StoreArgs,
    ) -> ClientResult<Vec<BlobStoreResult>>;

    /// Read a blob's bytes by id (Primary axis). See `WalrusNodeClient::read_blob`.
    async fn read_blob_primary(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>>;

    /// See `WalrusNodeClient::delete_owned_blob`.
    async fn delete_owned_blob(&self, blob_id: &BlobId) -> ClientResult<usize>;
}

// --- localnet impl: delegate to the inherent mirror methods ---

impl WalrusApi for crate::WalrusLocalClient {
    async fn reserve_and_store_blobs(
        &self,
        blobs: Vec<Vec<u8>>,
        store_args: &StoreArgs,
    ) -> ClientResult<Vec<BlobStoreResult>> {
        crate::WalrusLocalClient::reserve_and_store_blobs(self, blobs, store_args).await
    }

    async fn read_blob_primary(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>> {
        crate::WalrusLocalClient::read_blob_primary(self, blob_id).await
    }

    async fn delete_owned_blob(&self, blob_id: &BlobId) -> ClientResult<usize> {
        crate::WalrusLocalClient::delete_owned_blob(self, blob_id).await
    }
}

// --- real impl: PURE forwarding to walrus_sdk (no logic = no localnet/real-shared bug risk) ---

use walrus_core::encoding::Primary;
use walrus_sdk::node_client::{StoreBlobsApi, WalrusNodeClient};
use walrus_sui::client::SuiContractClient;

impl WalrusApi for WalrusNodeClient<SuiContractClient> {
    async fn reserve_and_store_blobs(
        &self,
        blobs: Vec<Vec<u8>>,
        store_args: &StoreArgs,
    ) -> ClientResult<Vec<BlobStoreResult>> {
        StoreBlobsApi::reserve_and_store_blobs(self, blobs, store_args).await
    }

    async fn read_blob_primary(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>> {
        self.read_blob::<Primary>(blob_id).await
    }

    async fn delete_owned_blob(&self, blob_id: &BlobId) -> ClientResult<usize> {
        WalrusNodeClient::delete_owned_blob(self, blob_id).await
    }
}
