// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Full nodeless WalrusStore round-trip against a LIVE suibase localnet that has a
//! Walrus deployment (descriptor present): store -> read -> stat -> extend -> delete.
//!
//! Gated by the env var `WALRUS_LOCALNET_TEST=1` so the default `cargo test` (which
//! has no running localnet) skips it cleanly. For CI, run it in an integration job
//! that has: started localnet, enabled walrus (walrus_local_enabled=true), and regen'd so
//! the deploy ran — then `WALRUS_LOCALNET_TEST=1 cargo test -p walrus-store \
//! --features localnet --test localnet_roundtrip`.

#![cfg(feature = "localnet")]

use walrus_store::WalrusStore;

#[tokio::test]
async fn localnet_roundtrip() -> anyhow::Result<()> {
    if std::env::var("WALRUS_LOCALNET_TEST").is_err() {
        eprintln!(
            "SKIP: set WALRUS_LOCALNET_TEST=1 with a live localnet + walrus deployment to run"
        );
        return Ok(());
    }

    let store = WalrusStore::for_workdir("localnet").await?;

    // Unique payload per run so we never collide with a prior blob id on-chain.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let payload = format!("hello nodeless walrus M2 round-trip {nonce}").into_bytes();

    // store
    let handle = store.store(&payload, 5).await?;
    eprintln!(
        "stored: blob_id={} object_id={}",
        handle.blob_id, handle.object_id
    );
    assert!(handle.object_id.starts_with("0x"));

    // re-store identical bytes -> content dedup: same blob_id AND same on-chain
    // object id (no duplicate Blob minted while the first is certified + unexpired).
    let handle2 = store.store(&payload, 5).await?;
    assert_eq!(handle2.blob_id, handle.blob_id, "same content -> same blob_id");
    assert_eq!(
        handle2.object_id, handle.object_id,
        "re-store must dedup to the existing Blob object, not mint a new one"
    );
    eprintln!("dedup OK (re-store returned existing object {})", handle2.object_id);

    // read
    let back = store.read(&handle.blob_id).await?;
    assert_eq!(back, payload, "read bytes != stored bytes");
    eprintln!("read OK ({} bytes)", back.len());

    // stat -> must be certified on-chain (off-node held-key certify worked)
    let meta = store.stat(&handle.blob_id).await?;
    eprintln!(
        "stat: size={} certified_epoch={:?} end_epoch={}",
        meta.size, meta.certified_epoch, meta.end_epoch
    );
    assert_eq!(meta.size, payload.len() as u64);
    assert!(
        meta.certified_epoch.is_some(),
        "blob is NOT certified — off-node certify did not take"
    );
    let end_before = meta.end_epoch;

    // extend -> hard-requires the certified state; must push end_epoch out
    store.extend(&handle.blob_id, 3).await?;
    let meta2 = store.stat(&handle.blob_id).await?;
    eprintln!("after extend: end_epoch {} -> {}", end_before, meta2.end_epoch);
    assert!(
        meta2.end_epoch > end_before,
        "extend did not increase end_epoch ({} -> {})",
        end_before,
        meta2.end_epoch
    );

    // delete -> burns the blob + removes local bytes
    store.delete(&handle.blob_id).await?;
    eprintln!("delete OK");
    assert!(
        store.read(&handle.blob_id).await.is_err(),
        "read should fail after delete (bytes removed)"
    );

    // delete is idempotent
    store.delete(&handle.blob_id).await?;

    eprintln!("M2 localnet round-trip: PASS");
    Ok(())
}
