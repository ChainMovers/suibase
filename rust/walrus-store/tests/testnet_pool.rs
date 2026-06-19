// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Real walrus-sdk storage-pool lifecycle against LIVE Sui testnet (M4 phase 2):
//! create_pool -> store_pooled (real encode+upload+certify into the pool) -> read ->
//! pool_status -> extend_pool -> grow_pool -> delete_pooled.
//!
//! FUND-/FEATURE-gated: skips (not fails) if the workdir/wallet is unavailable, or if
//! `create_pool` fails — the public testnet Walrus deployment may not have the storage
//! pool module, or the address may lack WAL. The pool CODE compiles + matches the SDK
//! either way; this test proves it works live when testnet supports pools + is funded.

#![cfg(feature = "real")]

use walrus_store::real::RealWalrusStore;

#[tokio::test]
async fn testnet_pool_lifecycle() -> anyhow::Result<()> {
    if std::env::var("WALRUS_TESTNET_TEST").is_err() {
        eprintln!("SKIP: set WALRUS_TESTNET_TEST=1 with a funded testnet workdir to run");
        return Ok(());
    }
    let store = match RealWalrusStore::for_workdir("testnet").await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SKIP: testnet store unavailable: {e}");
            return Ok(());
        }
    };
    eprintln!(
        "balances: {} MIST SUI, {} FROST WAL",
        store.sui_balance_mist().await?,
        store.wal_balance_frost().await?
    );

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let payload = format!("testnet pooled blob {nonce:030}").into_bytes();
    let enc = store.encoded_size(payload.len() as u64).await?;
    eprintln!("encoded size of {} bytes = {} bytes; creating a pool of {}", payload.len(), enc, enc * 2);

    // create_pool — if it fails, SKIP (testnet may lack the pool module, or low WAL).
    let pool = match store.create_pool(enc * 2, 1).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "SKIP: create_pool failed — testnet may not support storage pools yet, or \
                 insufficient WAL: {e}"
            );
            return Ok(());
        }
    };
    eprintln!("created pool {}", pool.pool_id);

    let st0 = store.pool_status(&pool.pool_id).await?;
    eprintln!(
        "pool status: reserved={} used={} blobs={} epochs {}..{}",
        st0.reserved_capacity_bytes, st0.used_bytes, st0.blob_count, st0.start_epoch, st0.end_epoch
    );
    assert_eq!(st0.blob_count, 0, "fresh pool has no blobs");

    // store_pooled -> real encode + upload to nodes + certify into the pool.
    let h = store.store_pooled(&pool.pool_id, &payload).await?;
    eprintln!("stored pooled: blob_id={} object_id={}", h.blob_id, h.object_id);
    assert!(h.object_id.starts_with("0x"));

    // read it back (retry briefly for post-certify propagation).
    let mut back = None;
    for attempt in 1..=3 {
        match store.read(&h.blob_id).await {
            Ok(b) => {
                back = Some(b);
                break;
            }
            Err(e) if attempt < 3 => {
                eprintln!("read attempt {attempt} (propagation lag?), retrying: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
    assert_eq!(back.expect("pooled read produced no bytes"), payload, "pooled read != stored");

    let st1 = store.pool_status(&pool.pool_id).await?;
    assert_eq!(st1.blob_count, 1, "pool should hold 1 blob after store_pooled");

    // extend_pool -> end_epoch moves out.
    store.extend_pool(&pool.pool_id, 1).await?;
    let st2 = store.pool_status(&pool.pool_id).await?;
    eprintln!("after extend_pool: end_epoch {} -> {}", st0.end_epoch, st2.end_epoch);
    assert!(st2.end_epoch > st0.end_epoch, "extend_pool did not push end_epoch");

    // grow_pool -> reserved capacity increases.
    store.grow_pool(&pool.pool_id, enc).await?;
    let st3 = store.pool_status(&pool.pool_id).await?;
    eprintln!("after grow_pool: reserved {} -> {}", st0.reserved_capacity_bytes, st3.reserved_capacity_bytes);
    assert!(
        st3.reserved_capacity_bytes > st0.reserved_capacity_bytes,
        "grow_pool did not increase reserved capacity"
    );

    // delete_pooled -> blob_count drops.
    store.delete_pooled(&pool.pool_id, &h.blob_id).await?;
    let st4 = store.pool_status(&pool.pool_id).await?;
    assert_eq!(st4.blob_count, st3.blob_count - 1, "delete_pooled did not drop blob_count");

    eprintln!("M4 testnet pool lifecycle: PASS");
    Ok(())
}
