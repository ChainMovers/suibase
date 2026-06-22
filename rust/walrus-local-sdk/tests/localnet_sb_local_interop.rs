// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Cross-front-door interop: a blob published through the `sb-local` HTTP **publisher** is
//! readable byte-for-byte through the Rust `WalrusLocalClient`, and a blob stored through
//! the Rust client is served byte-for-byte by the `sb-local` HTTP **aggregator** — and the
//! same content yields the same `blob_id` through both doors. This is the "two front doors,
//! one store" guarantee.
//!
//! Gated by `WALRUS_LOCALNET_TEST=1`; additionally requires `sb-local` to be up (it is,
//! when localnet is started with walrus enabled). `curl` is used as the HTTP client (same
//! as the bash wire test). Base URL overridable via `SB_LOCAL_BASE` (default
//! `http://localhost:45840`).

use std::process::Command;

use walrus_core::BlobId;
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::store_args::StoreArgs;

fn base() -> String {
    std::env::var("SB_LOCAL_BASE").unwrap_or_else(|_| "http://localhost:45840".to_string())
}

/// Run curl with proxy bypass; returns (success, stdout-bytes).
fn curl(args: &[&str]) -> (bool, Vec<u8>) {
    let mut full = vec!["-x", "", "-s"];
    full.extend_from_slice(args);
    let out = Command::new("curl").args(&full).output().expect("spawn curl");
    (out.status.success(), out.stdout)
}

#[tokio::test]
async fn sb_local_interop() -> anyhow::Result<()> {
    if std::env::var("WALRUS_LOCALNET_TEST").is_err() {
        eprintln!("SKIP: set WALRUS_LOCALNET_TEST=1 with a live localnet + walrus deployment to run");
        return Ok(());
    }
    let base = base();
    let (ok, body) = curl(&["-m", "3", &format!("{base}/status")]);
    if !ok || !String::from_utf8_lossy(&body).contains("OK") {
        eprintln!("SKIP: sb-local not reachable at {base} (start localnet with walrus_local_enabled=true)");
        return Ok(());
    }

    let client = WalrusLocalClient::for_workdir("localnet").await?;
    let args = StoreArgs::default_with_epochs(5);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Direction 1: store via the Rust SDK -> read via the HTTP aggregator.
    let p1 = format!("interop sdk->http {nonce}").into_bytes();
    let r1 = client.reserve_and_store_blobs(vec![p1.clone()], &args).await?;
    let id1 = r1[0].blob_id().expect("sdk store has a blob id");
    let (ok, got1) = curl(&["-m", "15", &format!("{base}/v1/blobs/{id1}")]);
    assert!(ok, "aggregator GET failed for {id1}");
    assert_eq!(got1, p1, "HTTP aggregator returned different bytes than the SDK stored");
    eprintln!("SDK store -> HTTP aggregate: OK ({id1})");

    // Direction 2: publish via the HTTP publisher -> read via the Rust SDK.
    let p2 = format!("interop http->sdk {nonce}").into_bytes();
    let tmp = std::env::temp_dir().join(format!("wlsdk-interop-{nonce}.bin"));
    std::fs::write(&tmp, &p2)?;
    let (ok, put_out) = curl(&[
        "-m", "30", "-X", "PUT",
        "--data-binary", &format!("@{}", tmp.display()),
        &format!("{base}/v1/blobs?epochs=5"),
    ]);
    let _ = std::fs::remove_file(&tmp);
    assert!(ok, "publisher PUT failed");
    let v: serde_json::Value = serde_json::from_slice(&put_out)
        .map_err(|e| anyhow::anyhow!("publisher response not JSON: {e}; body={}", String::from_utf8_lossy(&put_out)))?;
    let id2_str = v
        .pointer("/newlyCreated/blobObject/blobId")
        .or_else(|| v.pointer("/alreadyCertified/blobId"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow::anyhow!("no blobId in publisher response: {v}"))?;
    let id2: BlobId = id2_str.parse()?;
    let back = client.read_blob_primary(&id2).await?;
    assert_eq!(back, p2, "SDK read different bytes than the HTTP publisher stored");
    eprintln!("HTTP publish -> SDK read: OK ({id2})");

    // Direction 3: identical content -> identical blob_id across both front doors. Storing
    // p2 via the SDK dedups (content-addressed) to the same id the publisher returned.
    let r3 = client.reserve_and_store_blobs(vec![p2.clone()], &args).await?;
    assert_eq!(
        r3[0].blob_id(),
        Some(id2),
        "HTTP publisher and Rust SDK derive different blob_ids for identical content"
    );
    eprintln!("same content -> same blob_id across front doors: OK");

    // Cleanup (best-effort; the SDK p1 is Deletable, the HTTP p2 is Permanent so it stays).
    let _ = client.delete_owned_blob(&id1).await?;

    eprintln!("sb-local <-> walrus_local_sdk interop: PASS");
    Ok(())
}
