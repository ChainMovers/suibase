// Regression test for gRPC compressed-response rejection.
//
// A misbehaving upstream can ignore the proxy's `grpc-accept-encoding: identity`
// and answer a unary gRPC call with `grpc-encoding: gzip`. The Sui CLI's tonic
// client decodes only `identity` and would fail with
// `UNIMPLEMENTED: Content is compressed with gzip which isn't supported`. The
// proxy must therefore detect the unsupported response encoding and NOT forward
// it verbatim: it scores the attempt as a per-server failure (lowering that
// provider's Success% and health) and retries the next upstream.
//
// Determinism: like proxy_grpc_streaming_test.rs, ALL five mocks are configured
// identically (every one compresses), so no load-balancer rotation or retry
// ordering can let a "good" upstream answer first. With every upstream
// compressing, the client must NEVER receive a `grpc-encoding: gzip` response,
// and the providers' Success% must drop below 100.
//
// NOTE: when every upstream compresses, the proxy exhausts its retries and
// returns its OWN gRPC-level error (HTTP 200 + grpc-status, no grpc-encoding),
// so this test asserts on the ABSENCE of a non-identity grpc-encoding, not on
// HTTP status.

mod common;

use anyhow::{anyhow, Result};
use common::{
    clear_all_rate_limits, grpc_compress_behavior, reset_all_mock_servers, MockServerTestHarness,
};
use std::time::Duration;
use tokio::time::sleep;

const GET_SERVICE_INFO: &str = "/sui.rpc.v2.LedgerService/GetServiceInfo";
const MOCKS: [&str; 5] = ["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"];

#[tokio::test]
async fn test_proxy_rejects_compressed_grpc_response() -> Result<()> {
    let harness = MockServerTestHarness::new().await?;

    // Clean baseline: reset behaviors + rate limits, let state settle.
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    sleep(Duration::from_secs(2)).await;

    harness.ensure_servers_healthy(&MOCKS).await?;

    // Reset cumulative per-server stats so Success% reflects only this test.
    harness.reset_all_server_stats("localnet").await?;
    sleep(Duration::from_millis(500)).await;

    // Configure every mock to answer gRPC with `grpc-encoding: gzip`. No matter
    // which upstream the proxy selects or retries, it faces the compressed
    // response it must reject.
    for alias in MOCKS {
        harness
            .configure_mock_server(alias, grpc_compress_behavior())
            .await?;
    }
    // Wait until every mock has actually applied the compression behavior
    // rather than guessing with a fixed sleep (which can flake under CI load:
    // configure_mock_server can return before the behavior is observable).
    for alias in MOCKS {
        let mut applied = false;
        for _ in 0..50 {
            let mock_stats = harness.get_mock_server_stats(alias).await?;
            if mock_stats
                .current_behavior
                .as_ref()
                .map(|b| b.grpc_compress_response)
                .unwrap_or(false)
            {
                applied = true;
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
        assert!(
            applied,
            "mock '{alias}' did not apply grpc_compress_response within 5s"
        );
    }

    // Send a small burst of unary gRPC requests promptly after configuring
    // compression (before health-check cycles can demote the mocks out of
    // selection), so the per-attempt compression failures are recorded against
    // the providers. Each request is bounded so a hang fails fast — reqwest
    // returns once response headers arrive (HTTP/2).
    const N_REQUESTS: usize = 5;
    for i in 0..N_REQUESTS {
        let response = tokio::time::timeout(
            Duration::from_secs(10),
            harness.send_grpc_request(GET_SERVICE_INFO),
        )
        .await
        .map_err(|_| {
            anyhow!(
                "request {i}: proxy did not return response headers within 10s \
                 while every upstream compressed; it likely forwarded/awaited a \
                 gzip body it should have rejected."
            )
        })??;

        // The client must NEVER see a non-identity `grpc-encoding`. The proxy
        // either retried past the compressing upstreams and returned its own
        // (identity) error response, or — pre-fix — forwarded the gzip header
        // verbatim, which is exactly the bug. Absent or `identity` is correct.
        let client_encoding = response
            .headers()
            .get("grpc-encoding")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string());
        if let Some(enc) = client_encoding {
            assert!(
                enc.is_empty() || enc.eq_ignore_ascii_case("identity"),
                "request {i}: client received `grpc-encoding: {enc}` which the \
                 Sui CLI cannot decode; the proxy must not forward a compressed \
                 response verbatim"
            );
        }
        // Finite rejection-path body; nothing useful to read.
        drop(response);
    }

    // The compression must have been scored against the providers (lowering
    // Success%). Allow the failures to propagate to NetworkMonitor.
    sleep(Duration::from_secs(2)).await;
    let stats = harness.get_statistics("localnet").await?;
    let links = stats
        .links
        .ok_or_else(|| anyhow!("getLinks returned no links"))?;

    // `success_pct` is a String formatted "XX.XX" (or "" when a server received
    // no requests since the reset). At least one mock that handled a compressed
    // attempt must show < 100% success.
    let mut min_mock_pct: Option<f64> = None;
    for link in links.iter().filter(|l| l.alias.starts_with("mock-")) {
        if let Ok(pct) = link.success_pct.trim().parse::<f64>() {
            min_mock_pct = Some(min_mock_pct.map_or(pct, |m| m.min(pct)));
        }
    }

    let worst = min_mock_pct.ok_or_else(|| {
        anyhow!(
            "no mock provider reported a numeric success_pct after compressed \
             requests; expected at least one < 100"
        )
    })?;
    assert!(
        worst < 100.0,
        "expected at least one mock provider's Success% to drop below 100 after \
         returning compressed responses, but the lowest was {worst:.2}%"
    );

    Ok(())
}
