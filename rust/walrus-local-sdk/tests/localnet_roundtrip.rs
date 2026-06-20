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

use std::str::FromStr;

use walrus_core::encoding::Primary;
use walrus_core::encoding::quilt_encoding::{QuiltStoreBlob, QuiltVersionV1};
use walrus_core::{BlobId, EncodingType, QuiltPatchId};
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::responses::BlobStoreResult;
use walrus_sdk::node_client::store_args::StoreArgs;

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

    // --- extra mirror-surface coverage on FRESH blobs (the parity body deleted its own) ---
    let args = StoreArgs::default_with_epochs(5);
    let p1 = format!("mirror-extras-one-{nonce}").into_bytes();
    let p2 = format!("mirror-extras-two-{nonce}").into_bytes();
    let p3 = format!("mirror-extras-three-{nonce}").into_bytes();

    // multi-blob store: 3 inputs -> 3 results, in order, each NewlyCreated.
    let multi = client
        .reserve_and_store_blobs(vec![p1.clone(), p2.clone(), p3.clone()], &args)
        .await?;
    assert_eq!(multi.len(), 3, "3 inputs -> 3 BlobStoreResults (in order)");
    assert!(
        multi.iter().all(|r| matches!(r, BlobStoreResult::NewlyCreated { .. })),
        "each fresh multi-store result should be NewlyCreated"
    );
    let id1 = multi[0].blob_id().expect("result 0 has a blob id");

    // generic read success (turbofish) — proves U is accepted and the bytes round-trip.
    assert_eq!(client.read_blob::<Primary>(&id1).await?, p1, "generic read != stored");

    // get_blob_by_object_id: recover the on-chain Blob object id from NewlyCreated and
    // map it back to the same content id.
    let BlobStoreResult::NewlyCreated { blob_object, .. } = &multi[0] else {
        anyhow::bail!("expected NewlyCreated for a fresh store");
    };
    let bwa = client.get_blob_by_object_id(&blob_object.id).await?;
    assert_eq!(bwa.blob.blob_id, id1, "get_blob_by_object_id mapped to a different blob id");
    eprintln!("multi-store + get_blob_by_object_id + generic read: OK");

    // clean up the extras (default args => Deletable, so delete_owned_blob removes them).
    for r in &multi {
        assert_eq!(
            client.delete_owned_blob(&r.blob_id().unwrap()).await?,
            1,
            "deletable blob should be removed by delete_owned_blob"
        );
    }

    // Persistence parity: a PERMANENT blob is NOT removed by delete_owned_blob (the SDK
    // deletes only deletable owned blobs) -> returns 0 and the blob stays readable.
    let perm_args = StoreArgs::default_with_epochs(5).permanent();
    let pperm = format!("permanent-{nonce}").into_bytes();
    let pr = client.reserve_and_store_blobs(vec![pperm.clone()], &perm_args).await?;
    let pid = pr[0].blob_id().expect("permanent store has a blob id");
    assert_eq!(
        client.delete_owned_blob(&pid).await?,
        0,
        "delete_owned_blob must be a no-op for a Permanent blob"
    );
    assert_eq!(
        client.read_blob_primary(&pid).await?,
        pperm,
        "Permanent blob must survive delete_owned_blob"
    );
    eprintln!("persistence (Deletable vs Permanent delete semantics): OK");

    // Quilt round-trip through the mirror's sub-client (V1).
    let qc = client.quilt_client();
    let blobs = vec![
        QuiltStoreBlob::new_owned(b"alpha-bytes".to_vec(), "alpha")?
            .with_tags([("kind".to_string(), "a".to_string())]),
        QuiltStoreBlob::new_owned(format!("beta-{nonce}").into_bytes(), "beta")?,
    ];
    let quilt = qc.construct_quilt::<QuiltVersionV1>(&blobs, EncodingType::RS2).await?;
    let qres = qc.reserve_and_store_quilt::<QuiltVersionV1>(quilt, &args).await?;
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

    // Read by public QuiltPatchId (get_blobs_by_ids) — round-trips a stored patch id.
    let qpid = QuiltPatchId::from_str(&qres.stored_quilt_blobs[0].quilt_patch_id)?;
    let by_id = qc.get_blobs_by_ids(&[qpid]).await?;
    assert_eq!(by_id.len(), 1, "get_blobs_by_ids should return one patch");
    assert_eq!(
        by_id[0].identifier(),
        qres.stored_quilt_blobs[0].identifier,
        "get_blobs_by_ids returned a different patch than its id"
    );

    // Read by tag (get_blobs_by_tag) — only "alpha" carries kind=a.
    let tagged = qc.get_blobs_by_tag(&quilt_id, "kind", "a").await?;
    assert_eq!(tagged.len(), 1, "exactly one patch is tagged kind=a");
    assert_eq!(tagged[0].data(), b"alpha-bytes");
    eprintln!("quilt reads (by-id, by-tag) + multi-store: OK");

    eprintln!("localnet mirror round-trip + quilt: PASS");
    Ok(())
}
