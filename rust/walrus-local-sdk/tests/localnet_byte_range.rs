// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Extensive LIVE byte-range read coverage through the SDK-mirror sub-client
//! (`WalrusLocalClient::byte_range_read_client().read_byte_range`), mirroring
//! `walrus_sdk`'s `ByteRangeReadClient`. Stores one larger blob, then reads many
//! ranges (prefix / suffix / middle / single byte / boundaries) and compares each
//! against the in-memory payload, plus the SDK's exact input-error cases.
//!
//! Gated by `WALRUS_LOCALNET_TEST=1` like the other live suites.

use walrus_core::BlobId;
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::store_args::StoreArgs;

#[tokio::test]
async fn localnet_byte_range() -> anyhow::Result<()> {
    if std::env::var("WALRUS_LOCALNET_TEST").is_err() {
        eprintln!("SKIP: set WALRUS_LOCALNET_TEST=1 with a live localnet + walrus deployment to run");
        return Ok(());
    }

    let client = WalrusLocalClient::for_workdir("localnet").await?;

    // A multi-KB payload with a unique nonce header so the blob id is fresh per run,
    // and a deterministic body so range comparisons are meaningful. ~16 KiB.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut payload = format!("byte-range-{nonce:040}").into_bytes();
    payload.extend((0u32..4000).flat_map(|i| i.to_le_bytes()));
    let size = payload.len();
    assert!(size > 8_000, "payload should be multi-KB to exercise ranges");

    let args = StoreArgs::default_with_epochs(5);
    let results = client
        .reserve_and_store_blobs(vec![payload.clone()], &args)
        .await?;
    let blob_id: BlobId = results[0].blob_id().unwrap();
    eprintln!("stored {size}-byte blob {blob_id}");

    let brc = client.byte_range_read_client();

    // A broad set of (start, length) ranges, all in-bounds.
    let s = size as u64;
    let ranges: Vec<(u64, u64)> = vec![
        (0, s),            // whole blob
        (0, 1),            // first byte
        (s - 1, 1),        // last byte
        (0, 100),          // prefix
        (s - 100, 100),    // suffix ending exactly at len
        (1, s - 2),        // middle, almost-all
        (1000, 1),         // single middle byte
        (1234, 2345),      // arbitrary middle span
        (s / 2, s / 2),    // second half
        (s / 3, s / 3),    // a middle third
        (4096, 1),         // sliver-boundary-ish single
        (4096, 4096),      // sliver-sized span
        (7, 9),            // small odd-aligned span
        (s - 2, 2),        // last two bytes
    ];
    for (start, len) in ranges {
        let r = brc.read_byte_range(&blob_id, start, len).await?;
        let a = start as usize;
        let b = a + len as usize;
        assert_eq!(r.data, &payload[a..b], "data mismatch for range {start}+{len}");
        assert_eq!(r.unencoded_blob_size, s, "unencoded_blob_size for range {start}+{len}");
    }
    eprintln!("all in-bounds ranges matched the payload slices");

    // Error cases — must match walrus_sdk's ByteRangeReadInputError contract.
    assert!(brc.read_byte_range(&blob_id, s, 1).await.is_err(), "start == len must error");
    assert!(brc.read_byte_range(&blob_id, 0, s + 1).await.is_err(), "length past end must error");
    assert!(brc.read_byte_range(&blob_id, s - 1, 2).await.is_err(), "last byte + 1 must error");
    assert!(brc.read_byte_range(&blob_id, 0, 0).await.is_err(), "zero length must error");
    assert!(brc.read_byte_range(&blob_id, u64::MAX, 1).await.is_err(), "overflow must error");
    eprintln!("all out-of-bounds / invalid-input ranges errored as expected");

    // Clean up.
    let _ = client.delete_owned_blob(&blob_id).await?;

    eprintln!("localnet byte-range: PASS");
    Ok(())
}
