// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Regression test for the M3 review finding: identical content stored BOTH
//! standalone (`store`) and pooled (`store_pooled`) must not alias on disk. They
//! share one content-addressed `.bin` but keep independent sidecars (standalone vs
//! per-pool), so each on-chain object is indexed correctly and the shared bytes
//! survive until the LAST reference is deleted.
//!
//! Gated by `WALRUS_LOCALNET_TEST=1`. Lives in its own file (one test per binary) so
//! it runs sequentially w.r.t. the other live suites and never contends on the wallet.

use walrus_local_sdk::localnet::LocalnetMockStore;

#[tokio::test]
async fn standalone_and_pooled_same_content_coexist() -> anyhow::Result<()> {
    if std::env::var("WALRUS_LOCALNET_TEST").is_err() {
        eprintln!("SKIP: set WALRUS_LOCALNET_TEST=1 with a live localnet + walrus deployment");
        return Ok(());
    }

    let store = LocalnetMockStore::open().await?;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let payload = format!("coexist-same-content-{nonce:030}").into_bytes();

    // Standalone (Permanent) store of the content.
    let standalone = store.store(&payload, 5).await?;

    // A pool holding the SAME content.
    let enc = store.encoded_size(payload.len() as u64).await?;
    let pool = store.create_pool(enc * 2, 5).await?;
    let pooled = store.store_pooled(&pool.pool_id, &payload).await?;

    // Same content -> same blob_id, but DISTINCT on-chain objects.
    assert_eq!(standalone.blob_id, pooled.blob_id, "identical content -> same blob_id");
    assert_ne!(
        standalone.object_id, pooled.object_id,
        "standalone Blob and PooledBlob must be distinct on-chain objects"
    );

    // The standalone index is intact (NOT clobbered by the pooled store): stat reads
    // the Permanent Blob and it is still certified. (Pre-fix, store_pooled overwrote
    // the shared sidecar and this stat resolved to the PooledBlob and errored.)
    let meta = store.stat(&standalone.blob_id).await?;
    assert!(
        meta.certified_epoch.is_some(),
        "standalone blob must remain certified after a pooled store of identical content"
    );
    assert_eq!(store.read(&standalone.blob_id).await?, payload, "read != stored");

    // Deleting the standalone blob must NOT destroy the pooled blob's bytes: the
    // shared content-addressed bytes stay while the pooled reference remains.
    store.delete(&standalone.blob_id).await?;
    assert_eq!(
        store.read(&pooled.blob_id).await?,
        payload,
        "pooled bytes must survive deletion of the standalone blob"
    );

    // Deleting the pooled blob drops the last reference -> shared bytes are GC'd.
    store.delete_pooled(&pool.pool_id, &pooled.blob_id).await?;
    assert!(
        store.read(&pooled.blob_id).await.is_err(),
        "bytes should be gone once the last reference is deleted"
    );

    eprintln!("M3 namespace coexistence: PASS");
    Ok(())
}
