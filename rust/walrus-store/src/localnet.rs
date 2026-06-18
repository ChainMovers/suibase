// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Localnet nodeless mock for [`crate::WalrusStore`] (behind the `localnet` feature).
//!
//! Creates real `Blob`/`Storage` objects on the Suibase localnet Sui via PTBs +
//! off-node held-key `certify_blob`, with bytes served from the filesystem. There
//! are NO storage nodes: the bytes are written to disk keyed by the blob id, the
//! Merkle root is a deterministic stand-in (sha256 of the content), and the
//! confirmation certificate is built off-node from the held N=1 committee BLS key.
//!
//! Discovery:
//!   - the deploy-written descriptor `<workdir>/config/walrus-localnet.yaml`
//!     (package id + system/staking/treasury/exchange object ids + held committee
//!     BLS keypair), and
//!   - the workdir `client.yaml` (keystore + addresses), pinned to the direct
//!     fullnode RPC (`http://localhost:9000`) the same way the deploy bin does it.
//!
//! Built on top of the lightweight `suibase` helper (rust/helper) for workdir /
//! keystore / active-address resolution; the wallet itself is loaded by walrus-sui's
//! `load_wallet_context_from_path` against a sibling client.yaml pinned to 9000.
//!
//! IMPORTANT (public blob id): the id returned in [`crate::BlobHandle::blob_id`] is
//! the canonical Walrus `BlobId` string (URL-safe base64, no padding), e.g.
//! `E7_nNXvFU_3qZVu3OH1yycRG7LZlyn1-UxEDCDDqGGU`. The round-trip contract is what
//! matters and is honored: whatever `store` returns is accepted by
//! `read`/`stat`/`extend`/`delete`.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use sui_types::base_types::ObjectID;

use walrus_core::{
    encoding::{EncodingConfig, EncodingFactory},
    keys::ProtocolKeyPair,
    merkle::Node as MerkleNode,
    messages::{BlobPersistenceType, Confirmation, ConfirmationCertificate},
    BlobId, EncodingType, Epoch,
};
use walrus_sui::{
    client::{
        contract_config::ContractConfig, BlobObjectMetadata, BlobPersistence, PostStoreAction,
        ReadClient, SuiContractClient,
    },
    config::load_wallet_context_from_path,
    types::move_structs::BlobWithAttribute,
};

use crate::{BlobHandle, BlobMeta, PoolHandle, PoolStatus};

/// The single RS2 encoding type known to this walrus rev.
const ENCODING_TYPE: EncodingType = EncodingType::RS2;

/// Direct fullnode RPC of a suibase localnet (env alias `localnet` in client.yaml).
/// The default `active_env` is `localnet_proxy` (port 44340); we must talk to the
/// direct node so dry-run/simulate + object reads work without the proxy.
const LOCALNET_DIRECT_RPC: &str = "http://localhost:9000";

/// 1 SUI in MIST.
const ONE_SUI_MIST: u64 = 1_000_000_000;
/// SUI to swap for WAL on the first store of a process (exchange mints WAL 1:1).
/// Enough for many small-blob stores without re-funding; modest so repeated dev
/// processes don't meaningfully drain the faucet-funded SUI on a regen-able chain.
const WAL_FUNDING_SUI_MIST: u64 = 100 * ONE_SUI_MIST;

// ---------------------------------------------------------------------------
// Descriptor (deploy-written) — `<workdir>/config/walrus-localnet.yaml`
// ---------------------------------------------------------------------------

/// The suibase nodeless descriptor written by `walrus-localnet-deploy`.
///
/// `opt()` in the deploy bin emits the unquoted literal `null` for `None`, which
/// serde_yaml parses as `Value::Null -> Option::None`. We additionally normalize a
/// stray string `"null"` to `None` defensively (see `LocalnetDescriptor::load`).
#[derive(Debug, Clone, Deserialize)]
struct LocalnetDescriptor {
    #[allow(dead_code)]
    chain_id: String,
    #[allow(dead_code)]
    epoch: u32,
    #[allow(dead_code)]
    package_id: String,
    system_object: String,
    staking_object: String,
    #[serde(default)]
    #[allow(dead_code)]
    wal_exchange_pkg_id: Option<String>,
    #[serde(default)]
    exchange_object: Option<String>,
    #[serde(default)]
    treasury_object: Option<String>,
    #[allow(dead_code)]
    n_shards: u16,
    /// Held N=1 committee BLS keypair, flag||scalar Base64. LOCALNET ONLY.
    committee_protocol_keypair: String,
}

impl LocalnetDescriptor {
    fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading walrus descriptor {}", path.display()))?;
        let mut d: LocalnetDescriptor = serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing walrus descriptor {}", path.display()))?;
        // Defensive: treat a literal string "null" the same as absent.
        let denull = |o: Option<String>| o.filter(|s| s != "null" && !s.is_empty());
        d.wal_exchange_pkg_id = denull(d.wal_exchange_pkg_id);
        d.exchange_object = denull(d.exchange_object);
        d.treasury_object = denull(d.treasury_object);
        Ok(d)
    }
}

// ---------------------------------------------------------------------------
// On-disk sidecar: records what stat/extend/delete need without re-deriving.
// ---------------------------------------------------------------------------

/// Per-blob sidecar written next to the bytes (`<data_dir>/<hex>.meta`). Lets
/// stat/extend/delete map the public blob id -> on-chain ObjectID + size without
/// re-querying or re-deriving anything. The chain remains the source of truth for
/// epochs (stat re-reads them); the sidecar is the blob_id -> object_id index.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
struct BlobSidecar {
    /// Canonical Walrus BlobId string (URL-safe base64) — the public id.
    blob_id: String,
    /// On-chain object id (`0x` + hex): a `Blob` for a standalone blob, or a
    /// `PooledBlob` for a pooled one.
    object_id: String,
    /// Unencoded size in bytes.
    size: u64,
    /// Epochs purchased at store time (informational; chain is authoritative).
    /// Zero for pooled blobs — the owning pool holds the storage term.
    epochs: u32,
    /// For a pooled blob, the owning `StoragePool` object id; `None` for a
    /// standalone blob. Pooled sidecars also live under a per-pool subdirectory, so
    /// the on-disk path is the source of truth for scoping; this is a redundant
    /// in-file record for debuggability.
    #[serde(default)]
    pool_id: Option<String>,
}

// ---------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------

/// Nodeless localnet Walrus store backed by the deploy-written descriptor.
pub struct LocalnetMockStore {
    /// One contract client against localhost:9000; inner mutable state is behind a
    /// `Mutex` inside the client, so every op below takes `&self`.
    client: SuiContractClient,
    /// Held committee BLS keypair (N=1 committee, signer index 0).
    held_key: ProtocolKeyPair,
    /// Descriptor's SUI->WAL exchange object id (for first-store WAL funding).
    exchange_object: Option<ObjectID>,
    /// Directory holding `<hex>.bin` bytes + `<hex>.meta` sidecars.
    data_dir: PathBuf,
    /// Guards WAL funding: an async mutex held across the check + swap so concurrent
    /// stores cannot double-swap. The bool is "have we funded this process".
    wal_funded: tokio::sync::Mutex<bool>,
}

impl LocalnetMockStore {
    /// Open the localnet store: resolve the workdir via the suibase helper, read the
    /// deploy descriptor + a 9000-pinned wallet, and connect to the deployed system.
    pub async fn open() -> Result<Self> {
        // --- (1) Resolve workdir paths via the lightweight suibase helper. -----
        let helper = suibase::Helper::new();
        if !helper
            .is_installed()
            .map_err(|e| anyhow!("suibase helper: {e}"))?
        {
            bail!("suibase is not installed (no ~/suibase workdirs)");
        }
        helper
            .select_workdir("localnet")
            .map_err(|e| anyhow!("suibase select_workdir(localnet): {e}"))?;
        let keystore = PathBuf::from(
            helper
                .keystore_pathname()
                .map_err(|e| anyhow!("suibase keystore_pathname: {e}"))?,
        );
        // keystore = <workdir>/config/sui.keystore
        let config_dir = keystore
            .parent()
            .context("keystore has no parent config/ dir")?
            .to_path_buf();
        let workdir = config_dir
            .parent()
            .context("config/ has no parent workdir")?
            .to_path_buf();

        let descriptor_path = config_dir.join("walrus-localnet.yaml");
        if !descriptor_path.exists() {
            bail!(
                "walrus localnet descriptor not found at {} — run `localnet regen` (or the \
                 walrus deploy) first",
                descriptor_path.display()
            );
        }
        let client_yaml = config_dir.join("client.yaml");

        // --- (2) Parse the descriptor. ----------------------------------------
        let desc = LocalnetDescriptor::load(&descriptor_path)?;

        // --- (3) Held committee key (round-trips from base64 via FromStr). -----
        let held_key = ProtocolKeyPair::from_str(&desc.committee_protocol_keypair)
            .map_err(|e| anyhow!("parsing committee_protocol_keypair: {e}"))?;

        // --- (4) A wallet pinned to the DIRECT fullnode RPC (not the proxy). ---
        // suibase's client.yaml defaults active_env to `localnet_proxy`; reproduce
        // the deploy bin's `direct_rpc_wallet`: select the env whose rpc matches.
        let deploy_tmp = workdir.join("config").join("walrus-mock-tmp");
        std::fs::create_dir_all(&deploy_tmp)
            .with_context(|| format!("creating {}", deploy_tmp.display()))?;
        let wallet_yaml = direct_rpc_wallet(&client_yaml, LOCALNET_DIRECT_RPC, &deploy_tmp)
            .context("preparing direct-rpc wallet")?;
        let wallet = load_wallet_context_from_path(Some(&wallet_yaml), None)
            .context("loading localnet mock wallet")?;

        // --- (5) ContractConfig from the descriptor object ids. ---------------
        let system_object = ObjectID::from_str(&desc.system_object).context("system_object")?;
        let staking_object = ObjectID::from_str(&desc.staking_object).context("staking_object")?;
        let treasury_object = match &desc.treasury_object {
            Some(t) => Some(ObjectID::from_str(t).context("treasury_object")?),
            None => None,
        };
        let contract_config = ContractConfig::new(system_object, staking_object)
            .with_treasury_object(treasury_object);

        let exchange_object = match &desc.exchange_object {
            Some(e) => Some(ObjectID::from_str(e).context("exchange_object")?),
            None => None,
        };

        // --- (6) Build the contract client. -----------------------------------
        // contract_config BY REFERENCE; backoff BY VALUE (Default inferred from
        // the param type so we don't need to name walrus_utils). gas_budget=None
        // dry-runs to estimate (fine on localnet's direct node).
        let client = SuiContractClient::new(
            wallet,
            &[LOCALNET_DIRECT_RPC],
            &contract_config,
            Default::default(),
            None,
            Duration::from_secs(30),
        )
        .await
        .context("constructing SuiContractClient against localnet")?;

        // --- (7) Filesystem data dir for bytes + sidecars. --------------------
        let data_dir = config_dir.join("walrus-localnet-blobs");
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating blob data dir {}", data_dir.display()))?;

        Ok(Self {
            client,
            held_key,
            exchange_object,
            data_dir,
            wal_funded: tokio::sync::Mutex::new(false),
        })
    }

    // ----- store -----------------------------------------------------------

    /// Store `bytes` for `epochs` epochs: reserve_space -> register_blobs(Permanent)
    /// -> off-node held-key certify -> certify_blobs(Keep). The bytes + sidecar are
    /// written to disk (keyed by the blob id) BEFORE certify, so a certified on-chain
    /// blob is never left without servable bytes.
    ///
    /// Idempotent on content: re-storing the exact same bytes returns the existing
    /// handle (matching real Walrus blob_id dedup) instead of minting a duplicate
    /// `Blob` — as long as the prior on-chain object is still certified + unexpired.
    ///
    /// NOTE: store() is NOT atomic across the reserve/register/certify transactions;
    /// a crash mid-store can orphan an uncertified `Blob` (+ the WAL paid). This is
    /// acceptable on a regen-able localnet with faucet-minted WAL and is not worth
    /// crash-recovery machinery for a dev mock.
    pub async fn store(&self, bytes: &[u8], epochs: u32) -> Result<BlobHandle> {
        let unencoded_size = bytes.len() as u64;

        // Deterministic 32-byte Merkle-root stand-in (no real slivers exist); the
        // SAME root_hash flows into both the blob_id and the metadata struct.
        let root32: [u8; 32] = Sha256::digest(bytes).into();
        let root_hash = MerkleNode::from(root32);
        let blob_id = BlobId::from_metadata(root_hash.clone(), ENCODING_TYPE, unencoded_size);

        // Content dedup: if we already stored these exact bytes and the on-chain
        // Blob is still certified + unexpired, return the existing handle instead of
        // minting a duplicate (re-writing the identical bytes is harmless).
        if let Some(side) = self.try_read_sidecar(blob_id)? {
            if let Ok(object_id) = ObjectID::from_str(&side.object_id) {
                if let Ok(bwa) = self
                    .client
                    .read_client()
                    .get_blob_by_object_id(&object_id)
                    .await
                {
                    let live_epoch = self.client.read_client().current_epoch().await.unwrap_or(0);
                    if bwa.blob.certified_epoch.is_some() && bwa.blob.storage.end_epoch > live_epoch
                    {
                        self.write_bytes(blob_id, bytes)?; // ensure bytes present
                        return Ok(BlobHandle {
                            blob_id: side.blob_id,
                            object_id: side.object_id,
                        });
                    }
                }
            }
        }

        self.ensure_wal_funded().await?;

        // encoded_size from the committee shard count + RS2.
        let n_shards = self
            .client
            .read_client()
            .n_shards()
            .await
            .context("reading n_shards")?;
        let encoded_size = EncodingConfig::new(n_shards)
            .get_for_type(ENCODING_TYPE)
            .encoded_blob_length(unencoded_size)
            .context("computing encoded blob length (blob too large or zero-symbol?)")?;

        let metadata = BlobObjectMetadata {
            blob_id,
            root_hash,
            unencoded_size,
            encoded_size,
            encoding_type: ENCODING_TYPE,
        };

        // Reserve, then register as Permanent.
        let storage = self
            .client
            .reserve_space(encoded_size, epochs)
            .await
            .context("reserve_space (is the wallet funded with WAL?)")?;
        let mut blobs = self
            .client
            .register_blobs(vec![(metadata, storage)], BlobPersistence::Permanent)
            .await
            .context("register_blobs(Permanent)")?;
        let blob = blobs
            .pop()
            .ok_or_else(|| anyhow!("register_blobs returned no Blob"))?;

        let blob_id_str = blob_id.to_string();
        let object_id_str = blob.id.to_string();

        // Persist bytes + sidecar BEFORE certify, so a certified blob always has
        // servable local bytes (the object_id is already known from register).
        self.write_bytes(blob_id, bytes)?;
        self.write_sidecar(
            blob_id,
            &BlobSidecar {
                blob_id: blob_id_str.clone(),
                object_id: object_id_str.clone(),
                size: unencoded_size,
                epochs,
                pool_id: None,
            },
        )?;

        // Permanent confirmation: serializes as a single 0u8 tag with NO object id,
        // so the signed message is a pure function of (epoch, blob_id) and is
        // independent of blob.id (verified in walrus_core's own encoding test).
        let epoch: Epoch = self
            .client
            .read_client()
            .current_epoch()
            .await
            .context("current_epoch")?;
        let confirmation = Confirmation::new(epoch, blob_id, BlobPersistenceType::Permanent);
        let signed = self.held_key.sign_message(&confirmation);
        let certificate =
            ConfirmationCertificate::from_signed_messages_and_indices(vec![signed], vec![0u16])
                .map_err(|e| anyhow!("building ConfirmationCertificate from held key: {e}"))?;

        // Certify on-chain; Keep retains the blob in the wallet (needed for
        // extend/delete later).
        let with_attr = BlobWithAttribute {
            blob: blob.clone(),
            attribute: None,
        };
        let _shared: HashMap<BlobId, ObjectID> = self
            .client
            .certify_blobs(&[(&with_attr, certificate)], PostStoreAction::Keep)
            .await
            .context("certify_blobs (single-signer N=1 quorum)")?;

        Ok(BlobHandle {
            blob_id: blob_id_str,
            object_id: object_id_str,
        })
    }

    // ----- read ------------------------------------------------------------

    /// Read a stored blob's bytes by `blob_id` (served from the filesystem).
    pub async fn read(&self, blob_id: &str) -> Result<Vec<u8>> {
        let id = parse_blob_id(blob_id)?;
        let path = self.bytes_path(id);
        std::fs::read(&path).with_context(|| {
            format!("reading blob bytes for {blob_id} at {}", path.display())
        })
    }

    // ----- stat ------------------------------------------------------------

    /// Metadata for a stored blob: object id + size from the sidecar, and the live
    /// certified_epoch + end_epoch re-read from chain by object id.
    pub async fn stat(&self, blob_id: &str) -> Result<BlobMeta> {
        let id = parse_blob_id(blob_id)?;
        let side = self.read_sidecar(id)?;
        let object_id = ObjectID::from_str(&side.object_id).context("sidecar object_id")?;

        let bwa = self
            .client
            .read_client()
            .get_blob_by_object_id(&object_id)
            .await
            .with_context(|| format!("fetching Blob object {object_id}"))?;
        let b = bwa.blob;

        Ok(BlobMeta {
            blob_id: side.blob_id,
            object_id: side.object_id,
            size: side.size,
            certified_epoch: b.certified_epoch,
            end_epoch: b.storage.end_epoch,
        })
    }

    // ----- extend ----------------------------------------------------------

    /// Extend a certified blob's lifetime by `epochs` epochs (a COUNT, not an
    /// absolute epoch). HARD-REQUIRES the blob be certified AND not yet expired
    /// (Move `assert_certified_not_expired`) — exercises the held-key certify path.
    pub async fn extend(&self, blob_id: &str, epochs: u32) -> Result<()> {
        let id = parse_blob_id(blob_id)?;
        let side = self.read_sidecar(id)?;
        let object_id = ObjectID::from_str(&side.object_id).context("sidecar object_id")?;
        self.client
            .extend_blob(object_id, epochs)
            .await
            .with_context(|| format!("extend_blob {object_id} by {epochs} epochs"))?;
        Ok(())
    }

    // ----- delete ----------------------------------------------------------

    /// Delete a blob and remove its filesystem bytes. Realistic path: the blob is
    /// stored Permanent, so we `burn` it (Move `blob::burn` has NO assertions and
    /// works pre- or post-expiry) rather than `delete_blob` (which requires a
    /// Deletable blob). Idempotent w.r.t. missing local files.
    pub async fn delete(&self, blob_id: &str) -> Result<()> {
        let id = parse_blob_id(blob_id)?;
        // Best-effort: if the sidecar is gone we treat delete as already done.
        let side = match self.try_read_sidecar(id)? {
            Some(s) => s,
            None => return Ok(()),
        };
        let object_id = ObjectID::from_str(&side.object_id).context("sidecar object_id")?;
        self.client
            .burn_blobs(&[object_id])
            .await
            .with_context(|| format!("burn_blobs {object_id}"))?;

        // Remove this blob's standalone index; drop the shared content-addressed bytes
        // only if no pooled blob still references the same content (idempotent).
        let _ = std::fs::remove_file(self.sidecar_path(id));
        self.gc_bytes_if_unreferenced(id);
        Ok(())
    }

    // ----- storage pools (M3) ---------------------------------------------

    /// Encoded size in bytes of an `unencoded_size`-byte blob under this
    /// deployment's shard count + RS2. Pool capacities are in encoded bytes, so
    /// callers use this to size a pool to the blobs they intend to register.
    pub async fn encoded_size(&self, unencoded_size: u64) -> Result<u64> {
        let n_shards = self
            .client
            .read_client()
            .n_shards()
            .await
            .context("reading n_shards")?;
        EncodingConfig::new(n_shards)
            .get_for_type(ENCODING_TYPE)
            .encoded_blob_length(unencoded_size)
            .context("computing encoded blob length (blob too large or zero-symbol?)")
    }

    /// Create a storage pool reserving `reserved_capacity_bytes` of ENCODED capacity
    /// for `epochs` epochs. The walrus-sui wrapper transfers the created `StoragePool`
    /// to the sender and returns its object id. Pays WAL (funded once per process).
    pub async fn create_pool(&self, reserved_capacity_bytes: u64, epochs: u32) -> Result<PoolHandle> {
        self.ensure_wal_funded().await?;
        let pool_id = self
            .client
            .create_storage_pool(reserved_capacity_bytes, epochs)
            .await
            .context("create_storage_pool (is the wallet funded with WAL?)")?;
        Ok(PoolHandle {
            pool_id: pool_id.to_string(),
        })
    }

    /// Store `bytes` into an existing pool: register (Deletable) -> off-node held-key
    /// certify_pooled_blobs -> bytes to fs. The pool's pre-reserved capacity pays for
    /// storage, so there is no per-blob WAL/reserve here.
    ///
    /// Bytes are written BEFORE certify (same servable-bytes invariant as
    /// [`Self::store`]). Registered as Deletable so individual blobs can be removed
    /// from the pool via [`Self::delete_pooled`]; the certify message therefore binds
    /// to the pooled blob's own object id (`blob_persistence_type()` handles this).
    /// The sidecar is written under the pool's subdir so the same content can be
    /// pooled in several pools (and/or stored standalone) without aliasing.
    ///
    /// NOTE: not content-idempotent within a pool — re-storing identical bytes into
    /// the SAME pool aborts at register, because the pool's blob table rejects the
    /// duplicate blob id. (Unlike [`Self::store`], which dedups identical content.)
    pub async fn store_pooled(&self, pool_id: &str, bytes: &[u8]) -> Result<BlobHandle> {
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        let unencoded_size = bytes.len() as u64;

        let root32: [u8; 32] = Sha256::digest(bytes).into();
        let root_hash = MerkleNode::from(root32);
        let blob_id = BlobId::from_metadata(root_hash.clone(), ENCODING_TYPE, unencoded_size);
        let encoded_size = self.encoded_size(unencoded_size).await?;

        let metadata = BlobObjectMetadata {
            blob_id,
            root_hash,
            unencoded_size,
            encoded_size,
            encoding_type: ENCODING_TYPE,
        };

        let pooled = self
            .client
            .register_pooled_blobs(pool, vec![metadata], BlobPersistence::Deletable)
            .await
            .context(
                "register_pooled_blobs(Deletable) (pool out of capacity, or this exact \
                 content already registered in the pool?)",
            )?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("register_pooled_blobs returned no PooledBlob"))?;

        let blob_id_str = blob_id.to_string();
        let object_id_str = pooled.id.to_string();

        // Persist bytes (shared, content-addressed) + a POOL-SCOPED sidecar BEFORE
        // certify (epochs=0: the pool owns the term).
        self.write_bytes(blob_id, bytes)?;
        self.write_pooled_sidecar(
            pool_id,
            blob_id,
            &BlobSidecar {
                blob_id: blob_id_str.clone(),
                object_id: object_id_str.clone(),
                size: unencoded_size,
                epochs: 0,
                pool_id: Some(pool_id.to_string()),
            },
        )?;

        // Off-node held-key certify. The signed message's epoch must equal the
        // CURRENT (committee) epoch at certify time — the chain asserts
        // `cert_epoch == system epoch` — so re-read it here rather than reusing
        // `registered_epoch` (register and certify are separate transactions, and an
        // epoch tick between them would otherwise invalidate the signature). For a
        // Deletable pooled blob the persistence binds to the blob's object id, which
        // `blob_persistence_type()` supplies.
        let epoch: Epoch = self
            .client
            .read_client()
            .current_epoch()
            .await
            .context("current_epoch")?;
        let confirmation = Confirmation::new(epoch, blob_id, pooled.blob_persistence_type());
        let signed = self.held_key.sign_message(&confirmation);
        let certificate =
            ConfirmationCertificate::from_signed_messages_and_indices(vec![signed], vec![0u16])
                .map_err(|e| anyhow!("building pooled ConfirmationCertificate: {e}"))?;
        self.client
            .certify_pooled_blobs(pool, &[(&pooled, certificate)])
            .await
            .context("certify_pooled_blobs (single-signer N=1 quorum)")?;

        Ok(BlobHandle {
            blob_id: blob_id_str,
            object_id: object_id_str,
        })
    }

    /// Delete a pooled blob from `pool_id` (no certify) and remove its fs bytes.
    /// Idempotent and POOL-SCOPED: a re-delete after this pool's sidecar is gone is a
    /// no-op (the on-chain `delete_pooled_blob` would otherwise abort on the missing
    /// blob). The shared content-addressed bytes are dropped only if no other pool /
    /// standalone blob still references the same content.
    pub async fn delete_pooled(&self, pool_id: &str, blob_id: &str) -> Result<()> {
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        let id = parse_blob_id(blob_id)?;
        // The per-pool sidecar is our record that THIS pool holds the blob; once it's
        // gone we've already deleted from this pool (mirrors `delete`'s idempotency).
        if self.try_read_pooled_sidecar(pool_id, id)?.is_none() {
            return Ok(());
        }
        self.client
            .delete_pooled_blob(pool, id)
            .await
            .with_context(|| format!("delete_pooled_blob {blob_id} from pool {pool_id}"))?;
        let _ = std::fs::remove_file(self.pooled_sidecar_path(pool_id, id));
        self.gc_bytes_if_unreferenced(id);
        Ok(())
    }

    /// Live status of a storage pool (epochs, encoded capacity reserved/used, count).
    pub async fn pool_status(&self, pool_id: &str) -> Result<PoolStatus> {
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        let s = self
            .client
            .storage_pool_status(pool)
            .await
            .with_context(|| format!("storage_pool_status {pool_id}"))?;
        Ok(PoolStatus {
            pool_id: pool_id.to_string(),
            start_epoch: s.start_epoch,
            end_epoch: s.end_epoch,
            reserved_capacity_bytes: s.reserved_encoded_capacity_bytes,
            used_bytes: s.used_encoded_bytes,
            blob_count: s.blob_count,
        })
    }

    /// Extend a pool's lifetime by `epochs` epochs (pays WAL).
    pub async fn extend_pool(&self, pool_id: &str, epochs: u32) -> Result<()> {
        self.ensure_wal_funded().await?;
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        self.client
            .extend_storage_pool(pool, epochs)
            .await
            .with_context(|| format!("extend_storage_pool {pool_id} by {epochs} epochs"))?;
        Ok(())
    }

    /// Grow a pool's reserved ENCODED capacity by `additional_capacity_bytes` (WAL).
    pub async fn grow_pool(&self, pool_id: &str, additional_capacity_bytes: u64) -> Result<()> {
        self.ensure_wal_funded().await?;
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        self.client
            .increase_storage_pool_capacity(pool, additional_capacity_bytes)
            .await
            .with_context(|| format!("increase_storage_pool_capacity {pool_id}"))?;
        Ok(())
    }

    // ----- WAL funding -----------------------------------------------------

    /// Fund the active address with WAL once per process, via the descriptor's
    /// SUI->WAL exchange. `reserve_space` does NOT auto-fund (verified), so without
    /// this the first store aborts with insufficient WAL.
    async fn ensure_wal_funded(&self) -> Result<()> {
        // Hold the async lock across the check + swap so concurrent stores cannot
        // both observe `false` and double-swap.
        let mut funded = self.wal_funded.lock().await;
        if *funded {
            return Ok(());
        }
        let exchange_id = self.exchange_object.ok_or_else(|| {
            anyhow!("descriptor has no exchange_object; cannot mint WAL on localnet")
        })?;
        self.client
            .exchange_sui_for_wal(exchange_id, WAL_FUNDING_SUI_MIST)
            .await
            .context("exchange_sui_for_wal (funding WAL for store)")?;
        *funded = true;
        Ok(())
    }

    // ----- filesystem helpers ---------------------------------------------
    //
    // Blob BYTES are content-addressed and SHARED: keyed by blob_id at the top of
    // `data_dir`, so identical content stored several ways (standalone + pooled in one
    // or more pools) maps to a single `.bin`. The blob_id -> on-chain-object INDEX
    // (sidecar) is NOT 1:1, though — the same content can back a standalone `Blob` and
    // a `PooledBlob` in each of several pools — so sidecars are SCOPED: standalone at
    // `<hex>.meta`, pooled under `pools/<pool_id>/<hex>.meta`. Shared bytes are removed
    // only once the last sidecar referencing that blob_id is gone (`blob_id_referenced`).

    fn bytes_path(&self, id: BlobId) -> PathBuf {
        self.data_dir.join(format!("{}.bin", hex_key(id)))
    }
    fn sidecar_path(&self, id: BlobId) -> PathBuf {
        self.data_dir.join(format!("{}.meta", hex_key(id)))
    }
    /// Per-pool subdir holding that pool's pooled-blob sidecars. `pool_id` is
    /// `0x` + hex (filesystem-safe), so it is a valid directory name.
    fn pool_dir(&self, pool_id: &str) -> PathBuf {
        self.data_dir.join("pools").join(pool_id)
    }
    fn pooled_sidecar_path(&self, pool_id: &str, id: BlobId) -> PathBuf {
        self.pool_dir(pool_id).join(format!("{}.meta", hex_key(id)))
    }

    fn write_bytes(&self, id: BlobId, bytes: &[u8]) -> Result<()> {
        let path = self.bytes_path(id);
        std::fs::write(&path, bytes)
            .with_context(|| format!("writing blob bytes {}", path.display()))
    }

    fn write_sidecar_at(&self, path: &Path, side: &BlobSidecar) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating sidecar dir {}", parent.display()))?;
        }
        let yaml = serde_yaml::to_string(side).context("serializing sidecar")?;
        std::fs::write(path, yaml).with_context(|| format!("writing sidecar {}", path.display()))
    }
    fn try_read_sidecar_at(&self, path: &Path) -> Result<Option<BlobSidecar>> {
        match std::fs::read_to_string(path) {
            Ok(raw) => Ok(Some(
                serde_yaml::from_str(&raw)
                    .with_context(|| format!("parsing sidecar {}", path.display()))?,
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading sidecar {}", path.display())),
        }
    }

    // Standalone (non-pooled) sidecar accessors.
    fn write_sidecar(&self, id: BlobId, side: &BlobSidecar) -> Result<()> {
        self.write_sidecar_at(&self.sidecar_path(id), side)
    }
    fn read_sidecar(&self, id: BlobId) -> Result<BlobSidecar> {
        self.try_read_sidecar(id)?
            .ok_or_else(|| anyhow!("no sidecar for blob {} (unknown blob id)", id))
    }
    fn try_read_sidecar(&self, id: BlobId) -> Result<Option<BlobSidecar>> {
        self.try_read_sidecar_at(&self.sidecar_path(id))
    }

    // Pooled sidecar accessors (scoped to one pool).
    fn write_pooled_sidecar(&self, pool_id: &str, id: BlobId, side: &BlobSidecar) -> Result<()> {
        self.write_sidecar_at(&self.pooled_sidecar_path(pool_id, id), side)
    }
    fn try_read_pooled_sidecar(&self, pool_id: &str, id: BlobId) -> Result<Option<BlobSidecar>> {
        self.try_read_sidecar_at(&self.pooled_sidecar_path(pool_id, id))
    }

    /// True if ANY sidecar (standalone or in any pool) still references `id` — used
    /// to decide whether the shared content-addressed bytes may be removed.
    fn blob_id_referenced(&self, id: BlobId) -> bool {
        if self.sidecar_path(id).exists() {
            return true;
        }
        let needle = format!("{}.meta", hex_key(id));
        if let Ok(rd) = std::fs::read_dir(self.data_dir.join("pools")) {
            for entry in rd.flatten() {
                if entry.path().join(&needle).exists() {
                    return true;
                }
            }
        }
        false
    }

    /// Remove the shared bytes for `id` iff no sidecar references it any more.
    fn gc_bytes_if_unreferenced(&self, id: BlobId) {
        if !self.blob_id_referenced(id) {
            let _ = std::fs::remove_file(self.bytes_path(id));
        }
    }
}

// ---------------------------------------------------------------------------
// free helpers
// ---------------------------------------------------------------------------

/// Parse a public blob id (canonical Walrus URL-safe-base64 string) into a BlobId.
fn parse_blob_id(s: &str) -> Result<BlobId> {
    BlobId::from_str(s).map_err(|e| anyhow!("invalid blob id {s:?}: {e}"))
}

/// Filesystem-safe key for a BlobId: lowercase hex of its 32 bytes (avoids any
/// base64 characters in filenames). `BlobId(pub [u8; 32])` exposes its bytes.
fn hex_key(id: BlobId) -> String {
    let mut out = String::with_capacity(64);
    for b in id.0.iter() {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Reproduce the deploy bin's `direct_rpc_wallet`: read the suibase client.yaml,
/// find the env whose `rpc` matches `rpc`, pin `active_env` to its alias, and write
/// a sibling yaml. Suibase's default active_env is the proxy; we need the direct node.
fn direct_rpc_wallet(client_yaml: &Path, rpc: &str, out_dir: &Path) -> Result<PathBuf> {
    let raw = std::fs::read_to_string(client_yaml)
        .with_context(|| format!("reading {}", client_yaml.display()))?;
    let mut doc: serde_yaml::Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("parsing {}", client_yaml.display()))?;
    let alias = doc
        .get("envs")
        .and_then(|e| e.as_sequence())
        .into_iter()
        .flatten()
        .find(|env| env.get("rpc").and_then(|r| r.as_str()) == Some(rpc))
        .and_then(|env| env.get("alias").and_then(|a| a.as_str()))
        .map(|s| s.to_string())
        .with_context(|| format!("no env in {} has rpc {}", client_yaml.display(), rpc))?;
    doc["active_env"] = serde_yaml::Value::String(alias);
    std::fs::create_dir_all(out_dir)?;
    let out = out_dir.join("walrus_mock_wallet.yaml");
    std::fs::write(&out, serde_yaml::to_string(&doc)?)
        .with_context(|| format!("writing {}", out.display()))?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Pure-logic unit tests (no live localnet / no walrus-Sui graph at runtime).
// These exercise the blob-id key derivation, blob-id parsing, descriptor null
// normalization, and the direct-rpc wallet rewrite. They only touch /tmp, so
// they are safe to run in the per-push `cargo test --features localnet` (the
// live round-trip lives behind WALRUS_LOCALNET_TEST in tests/localnet_roundtrip).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch dir under the system temp dir (never the workdirs).
    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("walrus_store_ut_{}_{}", std::process::id(), tag));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn hex_key_is_64_char_lowercase_hex() {
        assert_eq!(hex_key(BlobId::ZERO), "0".repeat(64));
        assert_eq!(hex_key(BlobId::MAX), "f".repeat(64));

        let mut bytes = [0u8; 32];
        bytes[0] = 0x0a;
        bytes[31] = 0xbc;
        let key = hex_key(BlobId(bytes));
        assert_eq!(key.len(), 64);
        assert!(key.starts_with("0a") && key.ends_with("bc"), "key={key}");
        assert!(
            key.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "lowercase hex only: {key}"
        );
    }

    #[test]
    fn parse_blob_id_roundtrips_canonical_string() {
        let id = BlobId([7u8; 32]);
        let s = id.to_string(); // canonical URL-safe base64, no padding
        assert!(!s.contains('='), "canonical form is unpadded: {s}");
        assert_eq!(parse_blob_id(&s).expect("re-parse canonical blob id"), id);
    }

    #[test]
    fn parse_blob_id_rejects_garbage() {
        assert!(parse_blob_id("not a blob id").is_err()); // invalid chars
        assert!(parse_blob_id("").is_err()); // 0 bytes, not 32
        assert!(parse_blob_id("AAAA").is_err()); // valid base64, wrong length
    }

    #[test]
    fn descriptor_load_normalizes_bare_and_string_null() {
        let dir = tmp_dir("descriptor");
        let path = dir.join("walrus-localnet.yaml");
        std::fs::write(
            &path,
            concat!(
                "chain_id: abc123\n",
                "epoch: 1\n",
                "package_id: \"0x1\"\n",
                "system_object: \"0x2\"\n",
                "staking_object: \"0x3\"\n",
                "wal_exchange_pkg_id: \"0xwal\"\n",
                "exchange_object: null\n",      // bare null -> None
                "treasury_object: \"null\"\n",  // string "null" -> None (defensive denull)
                "n_shards: 1000\n",
                "committee_protocol_keypair: SOME_BASE64\n",
            ),
        )
        .unwrap();

        let d = LocalnetDescriptor::load(&path).expect("load descriptor");
        assert_eq!(d.system_object, "0x2");
        assert_eq!(d.staking_object, "0x3");
        assert_eq!(d.n_shards, 1000);
        assert_eq!(d.wal_exchange_pkg_id.as_deref(), Some("0xwal"));
        assert_eq!(d.exchange_object, None, "bare null -> None");
        assert_eq!(d.treasury_object, None, "string \"null\" -> None");
        assert_eq!(d.committee_protocol_keypair, "SOME_BASE64");
    }

    #[test]
    fn direct_rpc_wallet_pins_active_env_to_matching_rpc() {
        let dir = tmp_dir("wallet_ok");
        let client_yaml = dir.join("client.yaml");
        std::fs::write(
            &client_yaml,
            concat!(
                "active_env: localnet_proxy\n",
                "envs:\n",
                "  - alias: localnet\n",
                "    rpc: \"http://localhost:9000\"\n",
                "  - alias: localnet_proxy\n",
                "    rpc: \"http://localhost:44340\"\n",
            ),
        )
        .unwrap();

        let out = direct_rpc_wallet(&client_yaml, "http://localhost:9000", &dir.join("out"))
            .expect("rewrite wallet");
        let doc: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(doc["active_env"].as_str(), Some("localnet"));
        assert_eq!(doc["envs"].as_sequence().unwrap().len(), 2, "envs preserved");
    }

    #[test]
    fn direct_rpc_wallet_errors_when_no_env_matches() {
        let dir = tmp_dir("wallet_err");
        let client_yaml = dir.join("client.yaml");
        std::fs::write(
            &client_yaml,
            concat!(
                "active_env: localnet_proxy\n",
                "envs:\n",
                "  - alias: localnet_proxy\n",
                "    rpc: \"http://localhost:44340\"\n",
            ),
        )
        .unwrap();
        assert!(
            direct_rpc_wallet(&client_yaml, "http://localhost:9000", &dir.join("out")).is_err(),
            "no env matches the direct rpc -> error"
        );
    }
}