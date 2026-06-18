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

#[cfg(feature = "real")]
pub mod real;

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
    /// Epoch the pool's storage starts at.
    pub start_epoch: u32,
    /// Epoch after which the pool's storage expires.
    pub end_epoch: u32,
    /// Total reserved encoded capacity, in bytes.
    pub reserved_capacity_bytes: u64,
    /// Currently used encoded capacity, in bytes.
    pub used_bytes: u64,
    /// Number of pooled blobs currently registered in the pool.
    pub blob_count: u64,
}

/// Network-agnostic Walrus store. Construct with [`WalrusStore::for_workdir`].
///
/// One process may hold several independent stores (e.g. localnet + testnet) at once.
pub enum WalrusStore {
    #[cfg(feature = "localnet")]
    Localnet(localnet::LocalnetMockStore),
    #[cfg(feature = "real")]
    Real(real::RealWalrusStore),
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
            #[cfg(feature = "real")]
            "testnet" | "mainnet" => {
                Ok(Self::Real(real::RealWalrusStore::for_workdir(name).await?))
            }
            #[cfg(not(feature = "real"))]
            "testnet" | "mainnet" => bail!(
                "real walrus-sdk backend for '{name}' is not compiled in (enable the `real` feature)"
            ),
            other => bail!("unknown workdir '{other}'"),
        }
    }

    /// Store `bytes` for `epochs` epochs: creates a real, certified `Blob` on-chain
    /// (held-key certify) and writes the bytes to the filesystem.
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn store(&self, bytes: &[u8], epochs: u32) -> Result<BlobHandle> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.store(bytes, epochs).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.store(bytes, epochs).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Read a stored blob's bytes by `blob_id`.
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn read(&self, blob_id: &str) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.read(blob_id).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.read(blob_id).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Metadata for a stored blob.
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn stat(&self, blob_id: &str) -> Result<BlobMeta> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.stat(blob_id).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.stat(blob_id).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Extend a certified blob's lifetime by `epochs` (real `extend_blob`; requires
    /// the blob to be certified and not yet expired — exercises the held-key certify
    /// path end-to-end).
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn extend(&self, blob_id: &str, epochs: u32) -> Result<()> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.extend(blob_id, epochs).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.extend(blob_id, epochs).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Delete a blob (real `burn_blobs` on the `Blob` object) and remove its
    /// filesystem bytes. Idempotent: deleting an already-deleted blob is a no-op.
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn delete(&self, blob_id: &str) -> Result<()> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.delete(blob_id).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.delete(blob_id).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    // ----- storage pools -------------------------------------------------

    /// Encoded size, in bytes, of an `unencoded_size`-byte blob under this network's
    /// shard count + encoding. Use it to size pools (capacities are encoded bytes).
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn encoded_size(&self, unencoded_size: u64) -> Result<u64> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.encoded_size(unencoded_size).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.encoded_size(unencoded_size).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Create a storage pool reserving `reserved_capacity_bytes` of **encoded**
    /// capacity for `epochs` epochs (pays WAL up front). Pooled blobs registered
    /// into it draw down this shared capacity instead of each reserving their own.
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn create_pool(&self, reserved_capacity_bytes: u64, epochs: u32) -> Result<PoolHandle> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.create_pool(reserved_capacity_bytes, epochs).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.create_pool(reserved_capacity_bytes, epochs).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Store `bytes` into an existing pool: register (deletable) + off-node held-key
    /// certify into the pool, with bytes written to the filesystem. The pool's
    /// pre-reserved capacity pays for storage (no per-blob WAL).
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn store_pooled(&self, pool_id: &str, bytes: &[u8]) -> Result<BlobHandle> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.store_pooled(pool_id, bytes).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.store_pooled(pool_id, bytes).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Delete a pooled blob from `pool_id` (no certify required) and remove its
    /// filesystem bytes. Idempotent: re-deleting an already-deleted blob is a no-op.
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn delete_pooled(&self, pool_id: &str, blob_id: &str) -> Result<()> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.delete_pooled(pool_id, blob_id).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.delete_pooled(pool_id, blob_id).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Live status of a storage pool (epochs, encoded capacity, blob count).
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn pool_status(&self, pool_id: &str) -> Result<PoolStatus> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.pool_status(pool_id).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.pool_status(pool_id).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Extend a pool's lifetime by `epochs` epochs (pays WAL).
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn extend_pool(&self, pool_id: &str, epochs: u32) -> Result<()> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.extend_pool(pool_id, epochs).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.extend_pool(pool_id, epochs).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }

    /// Grow a pool's reserved **encoded** capacity by `additional_capacity_bytes`
    /// (pays WAL).
    #[cfg_attr(not(any(feature = "localnet", feature = "real")), allow(unused_variables))]
    pub async fn grow_pool(&self, pool_id: &str, additional_capacity_bytes: u64) -> Result<()> {
        match self {
            #[cfg(feature = "localnet")]
            Self::Localnet(s) => s.grow_pool(pool_id, additional_capacity_bytes).await,
            #[cfg(feature = "real")]
            Self::Real(s) => s.grow_pool(pool_id, additional_capacity_bytes).await,
            #[cfg(not(any(feature = "localnet", feature = "real")))]
            _ => bail!("no backend compiled in"),
        }
    }
}
