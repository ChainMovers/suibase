// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Nodeless storage-pool lifecycle against a LIVE suibase localnet with a Walrus
//! deployment: create_pool -> store_pooled (off-node held-key certify into the pool)
//! -> read -> pool_status -> extend_pool -> grow_pool -> delete_pooled.
//!
//! Storage pools are a localnet-engine feature (not part of the walrus_sdk high-level
//! WalrusNodeClient surface the mirror tracks), so this drives the engine directly.
//!
//! Gated by `WALRUS_LOCALNET_TEST=1` exactly like `localnet_roundtrip.rs`, so the
//! default `cargo test` (no running localnet) skips it cleanly.

use walrus_local_sdk::localnet::LocalnetMockStore;

#[tokio::test]
async fn localnet_pool_lifecycle() -> anyhow::Result<()> {
    if std::env::var("WALRUS_LOCALNET_TEST").is_err() {
        eprintln!(
            "SKIP: set WALRUS_LOCALNET_TEST=1 with a live localnet + walrus deployment to run"
        );
        return Ok(());
    }

    let store = LocalnetMockStore::open().await?;

    // Two DIFFERENT payloads of EQUAL length (so their encoded sizes match and the
    // pool can be sized deterministically), unique per run to avoid blob-id reuse.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let payload_a = format!("pooled-blob-A-{nonce:030}").into_bytes();
    let payload_b = format!("pooled-blob-B-{nonce:030}").into_bytes();
    assert_eq!(payload_a.len(), payload_b.len(), "payloads must be equal length");

    // Size a pool to comfortably hold both blobs (encoded), with headroom.
    let enc = store.encoded_size(payload_a.len() as u64).await?;
    let reserved = enc * 3;
    let pool = store.create_pool(reserved, 5).await?;
    eprintln!("created pool {} (reserved {} encoded bytes)", pool.pool_id, reserved);
    assert!(pool.pool_id.starts_with("0x"));

    let st0 = store.pool_status(&pool.pool_id).await?;
    eprintln!(
        "pool status: reserved={} used={} blobs={} epochs {}..{}",
        st0.reserved_capacity_bytes, st0.used_bytes, st0.blob_count, st0.start_epoch, st0.end_epoch
    );
    assert_eq!(st0.reserved_capacity_bytes, reserved, "pool reserved != requested");
    assert_eq!(st0.blob_count, 0, "fresh pool has no blobs");

    // store_pooled A -> certified into the pool; bytes servable from fs.
    let a = store.store_pooled(&pool.pool_id, &payload_a).await?;
    eprintln!("stored pooled A: blob_id={} object_id={}", a.blob_id, a.object_id);
    assert!(a.object_id.starts_with("0x"));
    assert_eq!(store.read(&a.blob_id).await?, payload_a, "read A != stored A");

    let st1 = store.pool_status(&pool.pool_id).await?;
    assert_eq!(st1.blob_count, 1, "pool should hold 1 blob after first store");
    assert!(st1.used_bytes > 0, "used capacity should be > 0 after a store");

    // store_pooled B -> second blob shares the same pool.
    let b = store.store_pooled(&pool.pool_id, &payload_b).await?;
    assert_ne!(b.blob_id, a.blob_id, "different content -> different blob_id");
    assert_eq!(store.read(&b.blob_id).await?, payload_b, "read B != stored B");
    let st2 = store.pool_status(&pool.pool_id).await?;
    assert_eq!(st2.blob_count, 2, "pool should hold 2 blobs");
    eprintln!("pool holds {} blobs, used {} bytes", st2.blob_count, st2.used_bytes);

    // extend_pool -> end_epoch must move out.
    store.extend_pool(&pool.pool_id, 2).await?;
    let st3 = store.pool_status(&pool.pool_id).await?;
    eprintln!("after extend: end_epoch {} -> {}", st2.end_epoch, st3.end_epoch);
    assert!(st3.end_epoch > st2.end_epoch, "extend_pool did not push end_epoch");

    // grow_pool -> reserved capacity must increase.
    store.grow_pool(&pool.pool_id, enc).await?;
    let st4 = store.pool_status(&pool.pool_id).await?;
    eprintln!(
        "after grow: reserved {} -> {}",
        st2.reserved_capacity_bytes, st4.reserved_capacity_bytes
    );
    assert!(
        st4.reserved_capacity_bytes > st2.reserved_capacity_bytes,
        "grow_pool did not increase reserved capacity"
    );

    // delete_pooled A -> blob_count drops, A's bytes gone, B still present.
    store.delete_pooled(&pool.pool_id, &a.blob_id).await?;
    let st5 = store.pool_status(&pool.pool_id).await?;
    assert_eq!(st5.blob_count, st4.blob_count - 1, "delete_pooled did not drop blob_count");
    assert!(store.read(&a.blob_id).await.is_err(), "A bytes should be gone after delete");
    assert_eq!(store.read(&b.blob_id).await?, payload_b, "B must survive A's deletion");

    // delete_pooled is idempotent: re-deleting after the sidecar is gone is a no-op.
    store.delete_pooled(&pool.pool_id, &a.blob_id).await?;

    eprintln!("M3 localnet pool lifecycle: PASS");
    Ok(())
}
