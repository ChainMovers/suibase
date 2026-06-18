// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Workdir-aware Walrus client for Suibase.
//!
//! `WalrusStore` is a network-agnostic store API. The caller selects the target
//! network **explicitly by name** — there is no global "active" workdir lookup:
//!
//! ```no_run
//! # async fn f() -> anyhow::Result<()> {
//! use walrus_store::WalrusStore;
//! let store = WalrusStore::for_workdir("localnet").await?;   // nodeless mock
//! let handle = store.store(b"hello", 5).await?;
//! let bytes = store.read(&handle.blob_id).await?;
//! # Ok(()) }
//! ```
//!
//! Backends:
//! - **localnet** -> [`localnet::LocalnetMockStore`] (behind the `localnet`/`mock`
//!   feature): real `Blob`/`Storage` objects on the Suibase localnet Sui via PTBs +
//!   off-node held-key `certify_blob`; bytes served from the filesystem. No storage
//!   nodes. Reads the deploy-written descriptor + the workdir's `client.yaml`.
//! - **testnet/mainnet** -> the real `walrus-sdk` backend (M4; not yet implemented).
//!
//! WS7: the `localnet` feature (and the heavy walrus/Sui graph it pulls) is OFF by
//! default, so the default build of this crate is inert and nothing enclave-side
//! depends on it.

#[cfg(feature = "localnet")]
pub mod localnet;

use anyhow::{bail, Result};

/// Handle to a stored blob: the content id (`blob_id`) and the on-chain Sui object id.
#[derive(Debug, Clone)]
pub struct BlobHandle {
    /// Content id — the canonical Walrus `BlobId` string (URL-safe base64, no
    /// padding), stable for identical content. Pass it back to `read`/`stat`/etc.
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

/// Network-agnostic Walrus store. Construct with [`WalrusStore::for_workdir`].
///
/// One process may hold several independent stores (e.g. localnet + testnet) at once.
pub enum WalrusStore {
    #[cfg(feature = "localnet")]
    Localnet(localnet::LocalnetMockStore),
    // Real(real::RealWalrusStore) — M4 (walrus-sdk backend for testnet/mainnet).
}

impl WalrusStore {
    /// Construct a store for the named suibase workdir (explicit; never the global
    /// "active"). `localnet` -> the nodeless mock; `testnet`/`mainnet` -> the real
    /// walrus-sdk backend (M4).
    pub async fn for_workdir(name: &str) -> Result<Self> {
        match name {
            #[cfg(feature = "localnet")]
            "localnet" => Ok(Self::Localnet(localnet::LocalnetMockStore::open().await?)),
            #[cfg(not(feature = "localnet"))]
            "localnet" => bail!(
                "localnet WalrusStore is not compiled in (enable the `localnet` feature)"
            ),
            "testnet" | "mainnet" => {
                bail!("real walrus-sdk backend for '{name}' is not yet implemented (M4)")
            }
            other => bail!("unknown workdir '{other}'"),
        }
    }

    /// Store `bytes` for `epochs` epochs: creates a real, certified `Blob` on-chain
    /// (held-key certify) and writes the bytes to the filesystem.
    #[cfg_attr(not(feature = "localnet"), allow(unused_variables))]
    pub async fn store(&self, bytes: &[u8], epochs: u32) -> Result<BlobHandle> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.store(bytes, epochs).await,
            #[cfg(not(feature = "localnet"))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Read a stored blob's bytes by `blob_id`.
    #[cfg_attr(not(feature = "localnet"), allow(unused_variables))]
    pub async fn read(&self, blob_id: &str) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.read(blob_id).await,
            #[cfg(not(feature = "localnet"))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Metadata for a stored blob.
    #[cfg_attr(not(feature = "localnet"), allow(unused_variables))]
    pub async fn stat(&self, blob_id: &str) -> Result<BlobMeta> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.stat(blob_id).await,
            #[cfg(not(feature = "localnet"))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Extend a certified blob's lifetime by `epochs` (real `extend_blob`; requires
    /// the blob to be certified and not yet expired — exercises the held-key certify
    /// path end-to-end).
    #[cfg_attr(not(feature = "localnet"), allow(unused_variables))]
    pub async fn extend(&self, blob_id: &str, epochs: u32) -> Result<()> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.extend(blob_id, epochs).await,
            #[cfg(not(feature = "localnet"))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Delete a blob (real `burn_blobs` on the `Blob` object) and remove its
    /// filesystem bytes. Idempotent: deleting an already-deleted blob is a no-op.
    #[cfg_attr(not(feature = "localnet"), allow(unused_variables))]
    pub async fn delete(&self, blob_id: &str) -> Result<()> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.delete(blob_id).await,
            #[cfg(not(feature = "localnet"))]
            _ => bail!("no backend compiled in"),
        }
    }
}
