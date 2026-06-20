// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Full nodeless round-trip against a LIVE suibase localnet that has a Walrus
//! deployment (descriptor present), driven through the SDK-mirror surface
//! (`WalrusLocalClient` + `compat::WalrusApi`): store -> read -> dedup -> delete,
//! plus localnet-specific follow-up (read-after-delete fails, quilt round-trip).
//!
//! Gated by `WALRUS_LOCALNET_TEST=1` so the default `cargo test` (no running localnet)
//! skips cleanly. For CI, run it in an integration job that has: started localnet,
//! enabled walrus (walrus_local_enabled=true), and regen'd so the deploy ran — then
//! `WALRUS_LOCALNET_TEST=1 cargo test -p walrus-local-sdk --test localnet_roundtrip`.

mod common;

use walrus_core::encoding::Primary;
use walrus_core::{BlobId, EncodingType};
use walrus_core::encoding::quilt_encoding::{QuiltStoreBlob, QuiltVersionV1};
use walrus_local_sdk::WalrusLocalClient;

#[tokio::test]
async fn localnet_roundtrip() -> anyhow::Result<()> {
    if std::env::var("WALRUS_LOCALNET_TEST").is_err() {
        eprintln!("SKIP: set WALRUS_LOCALNET_TEST=1 with a live localnet + walrus deployment to run");
        return Ok(());
    }

    let client = WalrusLocalClient::for_workdir("localnet").await?;

    // Unique payload per run so we never collide with a prior blob id on-chain.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let payload = format!("hello walrus-local-sdk mirror round-trip {nonce}").into_bytes();

    // The backend-agnostic parity body (store/read/dedup/delete via compat::WalrusApi).
    let blob_id_str = common::parity_roundtrip(&client, &payload).await?;

    // Localnet follow-up: read must fail after delete (bytes removed). Uses the
    // generic inherent read_blob::<U> to exercise the turbofish surface too.
    let blob_id: BlobId = blob_id_str.parse()?;
    assert!(
        client.read_blob::<Primary>(&blob_id).await.is_err(),
        "read should fail after delete (bytes removed)"
    );

    // Quilt round-trip through the mirror's sub-client (V1).
    let qc = client.quilt_client();
    let blobs = vec![
        QuiltStoreBlob::new_owned(b"alpha-bytes".to_vec(), "alpha")?
            .with_tags([("kind".to_string(), "a".to_string())]),
        QuiltStoreBlob::new_owned(format!("beta-{nonce}").into_bytes(), "beta")?,
    ];
    let quilt = qc.construct_quilt::<QuiltVersionV1>(&blobs, EncodingType::RS2).await?;
    let store_args = walrus_sdk::node_client::store_args::StoreArgs::default_with_epochs(5);
    let qres = qc.reserve_and_store_quilt::<QuiltVersionV1>(quilt, &store_args).await?;
    let quilt_id = qres
        .blob_store_result
        .blob_id()
        .ok_or_else(|| anyhow::anyhow!("quilt store carried no blob id"))?;
    assert_eq!(qres.stored_quilt_blobs.len(), 2, "two patches expected");
    eprintln!("quilt stored: id={quilt_id} patches={}", qres.stored_quilt_blobs.len());

    // Read patches back by identifier + all-blobs.
    let by_ident = qc.get_blobs_by_identifiers(&quilt_id, &["alpha"]).await?;
    assert_eq!(by_ident.len(), 1);
    assert_eq!(by_ident[0].data(), b"alpha-bytes");
    let all = qc.get_all_blobs(&quilt_id).await?;
    assert_eq!(all.len(), 2, "get_all_blobs should return both patches");

    eprintln!("localnet mirror round-trip + quilt: PASS");
    Ok(())
}
