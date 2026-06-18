// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Real Walrus backend (testnet/mainnet) for [`crate::WalrusStore`], behind the
//! `real` feature. Wraps the high-level `walrus-sdk` client: `store` encodes + uploads
//! to real storage nodes and registers/certifies a real on-chain `Blob`; `read` fetches
//! and reconstructs from the nodes. Unlike the localnet mock there is NO filesystem and
//! NO held key — certification comes from the real storage-node committee.
//!
//! Wallet/RPC: loads the suibase workdir `client.yaml`. Its `active_env` (by default the
//! suibase proxy, e.g. testnet at `http://localhost:44342`) becomes the primary Sui RPC,
//! with the public fullnode appended as a fallback — so normal operation exercises (and
//! thus validates) the proxy. Storage-node traffic is direct HTTP to the nodes; the proxy
//! is not involved there.
//!
//! WS7: the `real` feature pulls `walrus-sdk` but NOT `suibase` — the workdir `client.yaml`
//! is located by convention (`$HOME/suibase/workdirs/<network>/config/client.yaml`), so a
//! downstream enclave linking the real backend does not pull the suibase helper.

use std::{num::NonZeroU16, path::PathBuf, str::FromStr, time::Duration};

use anyhow::{anyhow, bail, Context, Result};

use walrus_sdk::{
    config::ClientConfig,
    core::{
        encoding::{EncodingFactory, Primary},
        BlobId, DEFAULT_ENCODING,
    },
    node_client::{
        responses::{BlobStoreResult, EventOrObjectId, PooledBlobStoreResult},
        StoreArgs, StoreBlobsApi, StoreBlobsInStoragePoolApi, WalrusNodeClient,
    },
    store_optimizations::StoreOptimizations,
    sui::{
        client::{contract_config::ContractConfig, BlobPersistence, PostStoreAction, SuiContractClient},
        coin::CoinType,
        config::load_wallet_context_from_path,
    },
    ObjectID,
};

use crate::{BlobHandle, BlobMeta, PoolHandle, PoolStatus};

/// Public Walrus contract deployment per network (from the pinned walrus rev's
/// `setup/client_config_*.yaml`). These ids are public + stable for the deployment.
struct NetworkContracts {
    system_object: &'static str,
    staking_object: &'static str,
    n_shards: u16,
    max_epochs_ahead: u32,
    /// Public fullnode, appended after the suibase proxy as an RPC fallback.
    fullnode: &'static str,
    /// SUI->WAL exchange objects (testnet only; mainnet has none — buy WAL).
    exchange_objects: &'static [&'static str],
}

const TESTNET: NetworkContracts = NetworkContracts {
    system_object: "0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af",
    staking_object: "0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3",
    n_shards: 1000,
    max_epochs_ahead: 53,
    fullnode: "https://fullnode.testnet.sui.io:443",
    exchange_objects: &[
        "0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073",
        "0x19825121c52080bb1073662231cfea5c0e4d905fd13e95f21e9a018f2ef41862",
        "0x83b454e524c71f30803f4d6c302a86fb6a39e96cdfb873c2d1e93bc1c26a3bc5",
        "0x8d63209cf8589ce7aef8f262437163c67577ed09f3e636a9d8e0813843fb8bf1",
    ],
};

const MAINNET: NetworkContracts = NetworkContracts {
    system_object: "0x2134d52768ea07e8c43570ef975eb3e4c27a39fa6396bef985b5abc58d03ddd2",
    staking_object: "0x10b9d30c28448939ce6c4d6c6e0ffce4a7f8a4ada8248bdad09ef8b70e4a3904",
    n_shards: 1000,
    max_epochs_ahead: 53,
    fullnode: "https://fullnode.mainnet.sui.io:443",
    exchange_objects: &[],
};

/// Real testnet/mainnet Walrus store backed by the `walrus-sdk` high-level client.
pub struct RealWalrusStore {
    client: WalrusNodeClient<SuiContractClient>,
    #[allow(dead_code)]
    network: String,
    /// SUI->WAL exchange object ids from the network config (for `exchange_sui_for_wal`).
    exchange_objects: Vec<ObjectID>,
}

impl RealWalrusStore {
    /// Open the real store for `testnet`/`mainnet`: resolve the suibase workdir wallet,
    /// build the walrus-sdk client against the public contract deployment, routing Sui
    /// RPC through the workdir's `active_env` (the suibase proxy) first.
    pub async fn for_workdir(network: &str) -> Result<Self> {
        let contracts = match network {
            "testnet" => &TESTNET,
            "mainnet" => &MAINNET,
            other => bail!("RealWalrusStore: unsupported network '{other}'"),
        };

        // WS7: locate the workdir client.yaml by convention (no suibase crate dep).
        let home = std::env::var("HOME").context("HOME not set")?;
        let client_yaml = PathBuf::from(&home)
            .join("suibase/workdirs")
            .join(network)
            .join("config/client.yaml");
        if !client_yaml.exists() {
            bail!(
                "suibase {network} wallet not found at {} — is the {network} workdir set up?",
                client_yaml.display()
            );
        }

        let wallet = load_wallet_context_from_path(Some(&client_yaml), Some(Duration::from_secs(60)))
            .map_err(|e| anyhow!("loading {network} wallet from {}: {e}", client_yaml.display()))?;
        // The wallet's active_env rpc (suibase proxy) becomes the write client's primary.
        let proxy_rpc = wallet.get_rpc_url().to_string();

        // Build the client config from the embedded public contract ids.
        let mut cc = ContractConfig::new(
            ObjectID::from_str(contracts.system_object).context("system_object")?,
            ObjectID::from_str(contracts.staking_object).context("staking_object")?,
        );
        cc.n_shards = Some(NonZeroU16::new(contracts.n_shards).context("n_shards must be > 0")?);
        cc.max_epochs_ahead = Some(contracts.max_epochs_ahead);

        let exchange_objects: Vec<ObjectID> = contracts
            .exchange_objects
            .iter()
            .map(|s| ObjectID::from_str(s))
            .collect::<std::result::Result<_, _>>()
            .context("exchange_objects")?;

        let mut config = ClientConfig::new_from_contract_config(cc);
        // Proxy primary (validates the suibase proxy under real load), fullnode fallback.
        config.rpc_urls = vec![proxy_rpc, contracts.fullnode.to_string()];
        config.exchange_objects = exchange_objects.clone();

        let sui_client = config
            .new_contract_client(wallet, None)
            .await
            .map_err(|e| anyhow!("building SuiContractClient for {network}: {e}"))?;
        let client = WalrusNodeClient::new_contract_client_with_refresher(config, sui_client)
            .await
            .map_err(|e| anyhow!("building walrus-sdk client for {network}: {e}"))?;

        Ok(Self {
            client,
            network: network.to_string(),
            exchange_objects,
        })
    }

    // ----- core: store + read ---------------------------------------------

    /// Store `bytes` for `epochs` epochs: encode + upload to the storage nodes and
    /// register + certify a real on-chain `Blob` (Deletable, kept in the wallet).
    pub async fn store(&self, bytes: &[u8], epochs: u32) -> Result<BlobHandle> {
        let store_args = StoreArgs::new(
            DEFAULT_ENCODING,
            epochs,
            StoreOptimizations::all(),
            BlobPersistence::Deletable,
            PostStoreAction::Keep,
        );
        let results = self
            .client
            .reserve_and_store_blobs(vec![bytes.to_vec()], &store_args)
            .await
            .map_err(|e| anyhow!("reserve_and_store_blobs: {e}"))?;
        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("reserve_and_store_blobs returned no result"))?;

        match result {
            BlobStoreResult::NewlyCreated { blob_object, .. } => Ok(BlobHandle {
                blob_id: blob_object.blob_id.to_string(),
                object_id: blob_object.id.to_string(),
            }),
            BlobStoreResult::AlreadyCertified {
                blob_id,
                event_or_object,
                ..
            } => {
                let object_id = match event_or_object {
                    EventOrObjectId::Object(o) => o.to_string(),
                    EventOrObjectId::Event(_) => String::new(),
                };
                Ok(BlobHandle {
                    blob_id: blob_id.to_string(),
                    object_id,
                })
            }
            other => bail!("store did not produce a stored blob: {other:?}"),
        }
    }

    /// Read a blob's bytes by `blob_id` (fetch + erasure-reconstruct from the nodes).
    pub async fn read(&self, blob_id: &str) -> Result<Vec<u8>> {
        let id = BlobId::from_str(blob_id).map_err(|e| anyhow!("invalid blob id {blob_id:?}: {e}"))?;
        self.client
            .read_blob::<Primary>(&id)
            .await
            .map_err(|e| anyhow!("read_blob {blob_id}: {e}"))
    }

    // ----- extend + delete -------------------------------------------------

    /// Extend the lifetime of an owned blob with this `blob_id` by `epochs` epochs.
    pub async fn extend(&self, blob_id: &str, epochs: u32) -> Result<()> {
        let id = BlobId::from_str(blob_id).map_err(|e| anyhow!("invalid blob id {blob_id:?}: {e}"))?;
        let blob = self
            .client
            .deletable_blobs_by_id(&id)
            .await
            .map_err(|e| anyhow!("listing owned blobs for {blob_id}: {e}"))?
            .next()
            .ok_or_else(|| anyhow!("no owned blob found for {blob_id} to extend"))?;
        self.client
            .sui_client()
            .extend_blob(blob.id, epochs)
            .await
            .map_err(|e| anyhow!("extend_blob {}: {e}", blob.id))?;
        Ok(())
    }

    /// Delete all owned deletable blobs with this `blob_id`.
    pub async fn delete(&self, blob_id: &str) -> Result<()> {
        let id = BlobId::from_str(blob_id).map_err(|e| anyhow!("invalid blob id {blob_id:?}: {e}"))?;
        self.client
            .delete_owned_blob(&id)
            .await
            .map_err(|e| anyhow!("delete_owned_blob {blob_id}: {e}"))?;
        Ok(())
    }

    // ----- balances + SUI->WAL funding (used by the fund-gated tests) -----

    /// Active Sui address of the loaded wallet (the funded one).
    pub fn active_address(&self) -> String {
        self.client.sui_client().address().to_string()
    }

    /// SUI balance of the active address, in MIST.
    pub async fn sui_balance_mist(&self) -> Result<u64> {
        self.client
            .sui_client()
            .total_balance(CoinType::Sui)
            .await
            .map_err(|e| anyhow!("querying SUI balance: {e}"))
    }

    /// WAL balance of the active address, in FROST.
    pub async fn wal_balance_frost(&self) -> Result<u64> {
        self.client
            .sui_client()
            .total_balance(CoinType::Wal)
            .await
            .map_err(|e| anyhow!("querying WAL balance: {e}"))
    }

    /// Convert `sui_amount_mist` of SUI into WAL via the network's exchange (testnet
    /// only — mainnet has no faucet exchange). Used to self-fund WAL for tests.
    pub async fn exchange_sui_for_wal(&self, sui_amount_mist: u64) -> Result<()> {
        let exchange_id = *self
            .exchange_objects
            .first()
            .ok_or_else(|| anyhow!("no SUI->WAL exchange on '{}' (buy WAL instead)", self.network))?;
        self.client
            .sui_client()
            .exchange_sui_for_wal(exchange_id, sui_amount_mist)
            .await
            .map_err(|e| anyhow!("exchange_sui_for_wal: {e}"))?;
        Ok(())
    }

    // ----- metadata + encoded size ----------------------------------------

    /// Metadata for a blob you own: size + epochs read from the on-chain `Blob` object
    /// located by `blob_id`. Requires the blob to be owned by this wallet (`store`
    /// keeps it, so a blob you stored is statable). `BlobStatus` from the storage nodes
    /// omits size/end_epoch for deletable blobs, so we read the Sui object instead.
    pub async fn stat(&self, blob_id: &str) -> Result<BlobMeta> {
        let id = BlobId::from_str(blob_id).map_err(|e| anyhow!("invalid blob id {blob_id:?}: {e}"))?;
        let blob = self
            .client
            .deletable_blobs_by_id(&id)
            .await
            .map_err(|e| anyhow!("listing owned blobs for {blob_id}: {e}"))?
            .next()
            .ok_or_else(|| anyhow!("no owned blob found for {blob_id} (stat needs an owned blob)"))?;
        Ok(BlobMeta {
            blob_id: blob_id.to_string(),
            object_id: blob.id.to_string(),
            size: blob.size,
            certified_epoch: blob.certified_epoch,
            end_epoch: blob.storage.end_epoch,
        })
    }

    /// Encoded size, in bytes, of an `unencoded_size`-byte blob under this network's
    /// shard count + encoding (for sizing pools — capacities are encoded bytes).
    pub async fn encoded_size(&self, unencoded_size: u64) -> Result<u64> {
        self.client
            .encoding_config()
            .get_for_type(DEFAULT_ENCODING)
            .encoded_blob_length(unencoded_size)
            .context("computing encoded blob length")
    }
    /// Create a storage pool reserving `reserved_capacity_bytes` of ENCODED capacity
    /// for `epochs` epochs (pays WAL). Returns the pool object id.
    pub async fn create_pool(&self, reserved_capacity_bytes: u64, epochs: u32) -> Result<PoolHandle> {
        let pool_id = self
            .client
            .sui_client()
            .create_storage_pool(reserved_capacity_bytes, epochs)
            .await
            .map_err(|e| anyhow!("create_storage_pool: {e}"))?;
        Ok(PoolHandle {
            pool_id: pool_id.to_string(),
        })
    }

    /// Store `bytes` into an existing pool: encode + upload to the storage nodes and
    /// register + certify a pooled blob, drawing on the pool's pre-reserved capacity
    /// (the full real flow via walrus-sdk's `reserve_and_store_blobs_in_storage_pool`).
    pub async fn store_pooled(&self, pool_id: &str, bytes: &[u8]) -> Result<BlobHandle> {
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        // epochs_ahead is governed by the pool's term; encoding/persistence from here.
        let store_args = StoreArgs::new(
            DEFAULT_ENCODING,
            1,
            StoreOptimizations::all(),
            BlobPersistence::Deletable,
            PostStoreAction::Keep,
        );
        let results = self
            .client
            .reserve_and_store_blobs_in_storage_pool(vec![bytes.to_vec()], pool, &store_args)
            .await
            .map_err(|e| anyhow!("reserve_and_store_blobs_in_storage_pool: {e}"))?;
        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("pooled store returned no result"))?;
        match result {
            PooledBlobStoreResult::NewlyCreated { pooled_blob_object } => Ok(BlobHandle {
                blob_id: pooled_blob_object.blob_id.to_string(),
                object_id: pooled_blob_object.id.to_string(),
            }),
            PooledBlobStoreResult::Error {
                failure_phase,
                error_msg,
                ..
            } => bail!("pooled store failed in {failure_phase}: {error_msg}"),
        }
    }

    /// Delete a pooled blob from `pool_id` (no certify needed).
    pub async fn delete_pooled(&self, pool_id: &str, blob_id: &str) -> Result<()> {
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        let id = BlobId::from_str(blob_id).map_err(|e| anyhow!("invalid blob id {blob_id:?}: {e}"))?;
        self.client
            .sui_client()
            .delete_pooled_blob(pool, id)
            .await
            .map_err(|e| anyhow!("delete_pooled_blob {blob_id} from {pool_id}: {e}"))?;
        Ok(())
    }

    /// Live status of a storage pool (epochs, encoded capacity reserved/used, count).
    pub async fn pool_status(&self, pool_id: &str) -> Result<PoolStatus> {
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        let s = self
            .client
            .sui_client()
            .storage_pool_status(pool)
            .await
            .map_err(|e| anyhow!("storage_pool_status {pool_id}: {e}"))?;
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
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        self.client
            .sui_client()
            .extend_storage_pool(pool, epochs)
            .await
            .map_err(|e| anyhow!("extend_storage_pool {pool_id}: {e}"))?;
        Ok(())
    }

    /// Grow a pool's reserved ENCODED capacity by `additional_capacity_bytes` (WAL).
    pub async fn grow_pool(&self, pool_id: &str, additional_capacity_bytes: u64) -> Result<()> {
        let pool = ObjectID::from_str(pool_id).context("pool_id")?;
        self.client
            .sui_client()
            .increase_storage_pool_capacity(pool, additional_capacity_bytes)
            .await
            .map_err(|e| anyhow!("increase_storage_pool_capacity {pool_id}: {e}"))?;
        Ok(())
    }
}
