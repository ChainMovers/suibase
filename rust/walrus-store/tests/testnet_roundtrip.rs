// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Real walrus-sdk backend round-trip against LIVE Sui testnet (M4): store a blob to
//! real storage nodes + register/certify a real on-chain `Blob`, then read it back.
//!
//! FUND-GATED (per the project's autonomous-test rule): the test inspects the suibase
//! testnet active address and
//!   - SKIPS with a single warning if the testnet workdir/wallet is unavailable (e.g.
//!     GitHub CI, which has no suibase workdir) or the address is underfunded;
//!   - otherwise AUTO-CONVERTS a little SUI -> WAL via the testnet exchange as needed,
//!     then runs the store -> read round-trip.
//!
//! So it never fails for lack of funds — it just doesn't run.
//!
//! Compiles only under `--features real`. Sui RPC is routed through the suibase proxy
//! (the workdir's active_env), which also validates the proxy under real load.

#![cfg(feature = "real")]

use walrus_store::real::RealWalrusStore;

const ONE_SUI_MIST: u64 = 1_000_000_000;
/// Convert this much SUI -> WAL when the address has little/no WAL.
const SUI_TO_CONVERT_MIST: u64 = ONE_SUI_MIST; // 1 SUI
/// Treat the address as "has WAL" above this (FROST).
const MIN_WAL_FROST: u64 = 100_000_000; // 0.1 WAL
/// Keep at least this much SUI for gas.
const GAS_HEADROOM_MIST: u64 = 200_000_000; // 0.2 SUI

#[tokio::test]
async fn testnet_roundtrip() -> anyhow::Result<()> {
    // Open the real testnet store. Missing workdir/wallet (CI) or an unreachable RPC
    // (proxy down) -> skip, don't fail.
    let store = match RealWalrusStore::for_workdir("testnet").await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SKIP: testnet store unavailable (no workdir/wallet or RPC down): {e}");
            return Ok(());
        }
    };
    let addr = store.active_address();
    let sui = store.sui_balance_mist().await?;
    let wal = store.wal_balance_frost().await?;
    eprintln!("testnet address {addr}: {sui} MIST SUI, {wal} FROST WAL");

    // Ensure the address can pay for storage (WAL) + gas (SUI); auto-convert if needed.
    if wal < MIN_WAL_FROST {
        if sui < SUI_TO_CONVERT_MIST + GAS_HEADROOM_MIST {
            eprintln!(
                "SKIP: insufficient funds for the testnet store test — have {sui} MIST SUI / \
                 {wal} FROST WAL, need ~{} MIST SUI (to convert to WAL) + gas. Fund {addr} via \
                 the faucet, or send it WAL.",
                SUI_TO_CONVERT_MIST + GAS_HEADROOM_MIST
            );
            return Ok(());
        }
        eprintln!("WAL low; converting {SUI_TO_CONVERT_MIST} MIST SUI -> WAL via the exchange ...");
        store.exchange_sui_for_wal(SUI_TO_CONVERT_MIST).await?;
        eprintln!("WAL after conversion: {} FROST", store.wal_balance_frost().await?);
    } else if sui < GAS_HEADROOM_MIST {
        eprintln!("SKIP: WAL is sufficient but SUI for gas is too low ({sui} MIST). Fund {addr}.");
        return Ok(());
    }

    // ---- store -> read round-trip on real testnet ----
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let payload = format!("hello real walrus testnet M4 round-trip {nonce}").into_bytes();

    eprintln!("storing {} bytes for 1 epoch (uploads to real storage nodes) ...", payload.len());
    let handle = store.store(&payload, 1).await?;
    eprintln!(
        "stored: blob_id={} object_id={}\n  inspect the on-chain Blob: \
         https://suiscan.xyz/testnet/object/{}",
        handle.blob_id, handle.object_id, handle.object_id
    );
    assert!(handle.object_id.starts_with("0x"), "expected an on-chain Blob object id");

    // Read it back from the storage nodes (retry briefly for post-certify propagation).
    let mut back = None;
    for attempt in 1..=3 {
        match store.read(&handle.blob_id).await {
            Ok(b) => {
                back = Some(b);
                break;
            }
            Err(e) if attempt < 3 => {
                eprintln!("read attempt {attempt} failed (propagation lag?), retrying: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
    let back = back.expect("read produced no bytes");
    assert_eq!(back, payload, "read bytes != stored bytes");

    eprintln!("read OK ({} bytes)", back.len());

    // stat -> certified + correct size (read from the on-chain owned Blob). [M4 phase 2]
    let meta = store.stat(&handle.blob_id).await?;
    eprintln!(
        "stat: size={} certified_epoch={:?} end_epoch={}",
        meta.size, meta.certified_epoch, meta.end_epoch
    );
    assert_eq!(meta.size, payload.len() as u64, "stat size != stored size");
    assert!(meta.certified_epoch.is_some(), "blob is not certified on-chain");
    let end_before = meta.end_epoch;

    // extend -> must push end_epoch out (hard-requires certified + unexpired).
    store.extend(&handle.blob_id, 2).await?;
    let meta2 = store.stat(&handle.blob_id).await?;
    eprintln!("after extend: end_epoch {} -> {}", end_before, meta2.end_epoch);
    assert!(
        meta2.end_epoch > end_before,
        "extend did not increase end_epoch ({} -> {})",
        end_before, meta2.end_epoch
    );

    // delete -> the owned Blob object is removed, so stat can no longer find it.
    store.delete(&handle.blob_id).await?;
    eprintln!("delete OK");
    assert!(
        store.stat(&handle.blob_id).await.is_err(),
        "stat should fail after delete (owned blob gone)"
    );

    eprintln!("M4 testnet store/read/stat/extend/delete: PASS");
    Ok(())
}
