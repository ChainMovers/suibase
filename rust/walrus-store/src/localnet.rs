// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Localnet nodeless mock for [`crate::WalrusStore`] (behind the `localnet` feature).
//!
//! Creates real `Blob`/`Storage` objects on the Suibase localnet Sui via PTBs +
//! off-node held-key `certify_blob`, with bytes served from the filesystem. Discovers
//! the deployment from the deploy-written descriptor
//! (`workdirs/localnet/config-default/walrus-localnet.yaml`) + the workdir's
//! `client.yaml` — no storage nodes, no dependency on the suibase helper's heavy path.
//!
//! NOTE: work in progress (M2). The discovery + client wiring and the
//! store/read/stat/extend/delete operations are being filled in next, reusing the
//! Gate-0 spike (crates/walrus-sui/examples/localnet_nodeless_certify.rs).

use anyhow::{bail, Result};

use crate::{BlobHandle, BlobMeta};

/// Nodeless localnet Walrus store backed by the deploy-written descriptor.
pub struct LocalnetMockStore {
    // Filled in by `open()`: SuiContractClient (against localhost:9000), the held
    // committee ProtocolKeyPair, n_shards, and the on-disk blob data dir.
}

impl LocalnetMockStore {
    /// Open the localnet store: read the descriptor + workdir wallet and connect to
    /// the localnet Sui at the deployed contract ids.
    pub async fn open() -> Result<Self> {
        bail!("LocalnetMockStore is not implemented yet (M2 in progress)")
    }

    pub async fn store(&self, _bytes: &[u8], _epochs: u32) -> Result<BlobHandle> {
        bail!("store not implemented yet (M2 in progress)")
    }

    pub async fn read(&self, _blob_id: &str) -> Result<Vec<u8>> {
        bail!("read not implemented yet (M2 in progress)")
    }

    pub async fn stat(&self, _blob_id: &str) -> Result<BlobMeta> {
        bail!("stat not implemented yet (M2 in progress)")
    }

    pub async fn extend(&self, _blob_id: &str, _epochs: u32) -> Result<()> {
        bail!("extend not implemented yet (M2 in progress)")
    }

    pub async fn delete(&self, _blob_id: &str) -> Result<()> {
        bail!("delete not implemented yet (M2 in progress)")
    }
}
