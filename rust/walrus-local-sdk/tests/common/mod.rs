// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Shared, backend-agnostic parity body for the round-trip tests.
//!
//! The SAME generic function runs against `WalrusLocalClient` (localnet) and against
//! the real `walrus_sdk::WalrusNodeClient` (testnet) — that is the drop-in proof: if
//! the mirror's signatures/types drifted from the SDK, this would not compile, and if
//! its behavior drifted, this would not pass. It uses ONLY the `compat::WalrusApi`
//! trait surface so it is genuinely identical across backends.

use walrus_local_sdk::compat::WalrusApi;
use walrus_sdk::node_client::store_args::StoreArgs;

/// store -> read -> dedup(re-store) -> delete, using only the shared trait surface.
/// Returns the stored `blob_id` string (for any backend-specific follow-up assertions).
pub async fn parity_roundtrip<C: WalrusApi>(client: &C, payload: &[u8]) -> anyhow::Result<String> {
    let args = StoreArgs::default_with_epochs(5);

    // store
    let results = client
        .reserve_and_store_blobs(vec![payload.to_vec()], &args)
        .await?;
    assert_eq!(results.len(), 1, "one input -> one BlobStoreResult");
    let blob_id = results[0]
        .blob_id()
        .ok_or_else(|| anyhow::anyhow!("store result carried no blob id: {:?}", results[0]))?;
    eprintln!("stored: blob_id={blob_id}");

    // read back
    let back = client.read_blob_primary(&blob_id).await?;
    assert_eq!(back, payload, "read bytes != stored bytes");
    eprintln!("read OK ({} bytes)", back.len());

    // byte-range read (drop-in: same call shape, same ReadByteRangeResult on both backends)
    if payload.len() >= 4 {
        let start = 1u64;
        let len = (payload.len() as u64 - 2).max(1); // a middle slice, length >= 1
        let r = client.read_byte_range(&blob_id, start, len).await?;
        assert_eq!(
            r.data,
            &payload[start as usize..(start + len) as usize],
            "byte-range data != payload slice"
        );
        assert_eq!(
            r.unencoded_blob_size,
            payload.len() as u64,
            "byte-range unencoded_blob_size != full blob size"
        );
        // Out-of-bounds must error identically on both backends.
        assert!(
            client
                .read_byte_range(&blob_id, payload.len() as u64, 1)
                .await
                .is_err(),
            "byte range past end should error"
        );
        eprintln!("byte-range OK ({len} bytes at {start}, total {})", r.unencoded_blob_size);
    }

    // re-store identical bytes -> content dedup to the SAME blob id
    let results2 = client
        .reserve_and_store_blobs(vec![payload.to_vec()], &args)
        .await?;
    assert_eq!(
        results2[0].blob_id(),
        Some(blob_id),
        "re-store of identical content must dedup to the same blob id"
    );
    eprintln!("dedup OK");

    // delete (count >= 1 since we just stored it)
    let removed = client.delete_owned_blob(&blob_id).await?;
    assert!(removed >= 1, "delete should remove at least the blob we stored");
    eprintln!("delete OK (removed {removed})");

    // delete is idempotent
    let _ = client.delete_owned_blob(&blob_id).await?;

    Ok(blob_id.to_string())
}
