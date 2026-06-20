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
use walrus_sdk::node_client::responses::BlobStoreResult;
use walrus_sdk::node_client::store_args::StoreArgs;
use walrus_storage_node_client::api::BlobStatus;

/// A fixed (non-nonce'd) content fixture and its canonical Walrus `BlobId`. This exact id is
/// triple-confirmed for this content: the localnet store, the real-testnet store, AND the
/// `walrus` CLI computed *in the testnet context* (`walrus --context testnet blob-id`) all
/// produce it. (NB: `walrus blob-id --n-shards 1000` *standalone* gives a DIFFERENT value —
/// the standalone path derives the RS2 encoding params differently than the live committee;
/// the committee/store value is the authoritative one.) Storing this content MUST yield this
/// id on either backend — the cross-environment blob_id parity proof.
pub const BLOB_ID_FIXTURE: &[u8] = b"walrus-local-sdk cross-environment blob_id fixture v1";
pub const EXPECTED_FIXTURE_BLOB_ID: &str = "x37bth2QxQZBbjZS6F-6l9mU_-bp46CRfOo33IAwe2U";

/// store -> read -> dedup(re-store) -> delete, using only the shared trait surface.
/// Returns the stored `blob_id` string (for any backend-specific follow-up assertions).
pub async fn parity_roundtrip<C: WalrusApi>(client: &C, payload: &[u8]) -> anyhow::Result<String> {
    let args = StoreArgs::default_with_epochs(5);

    // Cross-environment blob_id parity: the SAME content yields the SAME canonical blob_id
    // on localnet AND testnet (both n_shards=1000 + walrus-core encoder), equal to the
    // `walrus blob-id` CLI output. Storing the fixed fixture and checking the id proves it
    // on whichever backend this body runs against.
    let fixture = client
        .reserve_and_store_blobs(vec![BLOB_ID_FIXTURE.to_vec()], &args)
        .await?;
    assert_eq!(
        fixture[0].blob_id().expect("fixture has a blob id").to_string(),
        EXPECTED_FIXTURE_BLOB_ID,
        "cross-environment blob_id mismatch (n_shards / encoder drift?)"
    );
    eprintln!("blob_id parity OK (fixture == {EXPECTED_FIXTURE_BLOB_ID})");

    // store
    let results = client
        .reserve_and_store_blobs(vec![payload.to_vec()], &args)
        .await?;
    assert_eq!(results.len(), 1, "one input -> one BlobStoreResult");
    // Unique (nonce'd) payload -> a fresh blob, so the first store is NewlyCreated on
    // BOTH backends (drop-in: the SDK variant, not just the id, matches).
    assert!(
        matches!(results[0], BlobStoreResult::NewlyCreated { .. }),
        "first store of fresh content should be NewlyCreated, got {:?}",
        results[0]
    );
    let blob_id = results[0]
        .blob_id()
        .ok_or_else(|| anyhow::anyhow!("store result carried no blob id: {:?}", results[0]))?;
    eprintln!("stored: blob_id={blob_id}");

    // read back
    let back = client.read_blob_primary(&blob_id).await?;
    assert_eq!(back, payload, "read bytes != stored bytes");
    eprintln!("read OK ({} bytes)", back.len());

    // blob_status parity: a freshly-stored Deletable blob (default StoreArgs) reports
    // Deletable + certified on BOTH backends — the localnet status is derived from chain,
    // so it must agree with testnet's node-quorum status for the same blob_id.
    match client.blob_status(&blob_id).await? {
        BlobStatus::Deletable { initial_certified_epoch, deletable_counts } => {
            assert!(initial_certified_epoch.is_some(), "stored blob should be certified");
            assert!(
                deletable_counts.count_deletable_total >= 1,
                "at least one deletable blob object expected"
            );
        }
        other => anyhow::bail!("expected Deletable blob_status for a deletable store, got {other:?}"),
    }
    eprintln!("blob_status parity OK (Deletable, certified)");

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
    // The just-stored, certified, unexpired blob dedups to AlreadyCertified on BOTH
    // backends (not NewlyCreated) — the wire-faithful variant a drop-in caller sees.
    assert!(
        matches!(results2[0], BlobStoreResult::AlreadyCertified { .. }),
        "re-store should dedup to AlreadyCertified, got {:?}",
        results2[0]
    );
    eprintln!("dedup OK (AlreadyCertified)");

    // delete (count >= 1 since we just stored it)
    let removed = client.delete_owned_blob(&blob_id).await?;
    assert!(removed >= 1, "delete should remove at least the blob we stored");
    eprintln!("delete OK (removed {removed})");

    // delete is idempotent
    let _ = client.delete_owned_blob(&blob_id).await?;

    Ok(blob_id.to_string())
}
