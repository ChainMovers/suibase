// Integration test for the gRPC dispatch path's QPS/QPM/RespT/health tracking.
//
// Mirrors `qps_qpm_tracking_test.rs` (which covers JSON-RPC) but drives the
// proxy with HTTP/2 + `content-type: application/grpc`. Verifies:
//
//   1. gRPC requests through the proxy succeed (200 + application/grpc)
//   2. QPS/QPM counters increment for the upstream that served the request
//   3. RespT (resp_time) is recorded once at least one probe round-trip lands
//   4. A mock answering JSON instead of gRPC is classified NOT_GRPC_CAPABLE
//      and force-downed (selectable -> false / health drops)

mod common;

use anyhow::Result;
use common::{
    clear_all_rate_limits, non_grpc_behavior, reset_all_mock_servers, MockServerTestHarness,
};
use std::time::Duration;
use tokio::time::sleep;

const GET_SERVICE_INFO: &str = "/sui.rpc.v2.LedgerService/GetServiceInfo";

#[tokio::test]
async fn test_grpc_burst_updates_qps_qpm_resptime() -> Result<()> {
    let harness = MockServerTestHarness::new().await?;

    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    sleep(Duration::from_secs(2)).await;

    harness
        .ensure_servers_healthy(&["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"])
        .await?;

    // Send a burst of gRPC requests through the proxy.
    let burst = 20;
    let responses = harness.send_grpc_burst(burst, GET_SERVICE_INFO).await?;

    // Every response should be HTTP 200 with a gRPC content-type. The mock
    // returns these regardless of which mock-N served the request; the
    // proxy forwards them as-is.
    let mut grpc_ok = 0;
    for r in &responses {
        let ct = r
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if r.status().is_success() && ct.starts_with("application/grpc") {
            grpc_ok += 1;
        }
    }
    assert!(
        grpc_ok > 0,
        "expected at least one successful gRPC response from the burst; got {} / {}",
        grpc_ok,
        burst
    );

    // Give NetworkMonitor a moment to consume the reports the proxy emitted.
    sleep(Duration::from_secs(1)).await;

    let stats = harness.get_statistics("localnet").await?;
    let links = stats
        .links
        .ok_or_else(|| anyhow::anyhow!("no links in statistics response"))?;

    // At least one mock-N must show non-zero QPS or QPM after the burst.
    let qps_or_qpm_seen = links
        .iter()
        .filter(|l| l.alias.starts_with("mock-"))
        .any(|l| matches!(l.qps_raw, Some(n) if n > 0) || matches!(l.qpm_raw, Some(n) if n > 0));
    assert!(
        qps_or_qpm_seen,
        "after a {}-request gRPC burst, no mock-N has non-zero QPS or QPM. links: {:#?}",
        burst, links
    );

    // RespT comes from the controlled latency probe (X-SBSD-SERVER-HC). The
    // gRPC dispatch now sets HEADER_SBSD_SERVER_HC_SET on those, so within a
    // few probe cycles at least one mock-N should have a numeric resp_time.
    // Probes run roughly every 15s; we already waited 2 + 1 second above —
    // wait a little longer to give one round a chance to land.
    let mut resp_time_seen = false;
    for _ in 0..30 {
        sleep(Duration::from_secs(1)).await;
        let stats = harness.get_statistics("localnet").await?;
        if let Some(links) = stats.links {
            if links
                .iter()
                .filter(|l| l.alias.starts_with("mock-"))
                .any(|l| !l.resp_time.is_empty() && l.resp_time != "-")
            {
                resp_time_seen = true;
                break;
            }
        }
    }
    assert!(
        resp_time_seen,
        "no mock-N produced a non-empty resp_time after waiting for the probe cycle"
    );

    Ok(())
}

#[tokio::test]
async fn test_grpc_marks_non_grpc_upstream_down() -> Result<()> {
    let harness = MockServerTestHarness::new().await?;

    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    sleep(Duration::from_secs(2)).await;

    harness
        .ensure_servers_healthy(&["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"])
        .await?;

    // Make mock-0 answer gRPC requests with a JSON body — this is exactly
    // what a public JSON-RPC-only gateway looks like to the proxy.
    harness
        .configure_mock_server("mock-0", non_grpc_behavior())
        .await?;

    // Send some gRPC traffic. The proxy will try targets in priority order;
    // when it hits mock-0 it should classify it NOT_GRPC_CAPABLE and demote
    // it from the gRPC selection. Per the F4 fix, mock-0 must remain
    // JSON-RPC-healthy — only gRPC dispatch should skip it.
    let _ = harness.send_grpc_burst(20, GET_SERVICE_INFO).await?;
    sleep(Duration::from_secs(1)).await;

    // Snapshot mock-0's request count BEFORE the post-demotion gRPC burst.
    let mock0_before = harness.get_mock_server_stats("mock-0").await?;

    // Send a second gRPC burst. mock-0 should be excluded from selection
    // now (is_grpc_capable=false) — its request counter must NOT increase.
    let _ = harness.send_grpc_burst(20, GET_SERVICE_INFO).await?;
    sleep(Duration::from_secs(1)).await;

    let mock0_after = harness.get_mock_server_stats("mock-0").await?;
    let new_requests = mock0_after
        .stats
        .requests_received
        .saturating_sub(mock0_before.stats.requests_received);
    assert_eq!(
        new_requests, 0,
        "mock-0 (marked NOT_GRPC_CAPABLE) should receive 0 new gRPC requests after demotion; got {}",
        new_requests
    );

    // mock-0 must remain JSON-RPC-healthy. is_healthy is consulted by the
    // JSON-RPC selection_vectors; per F4 the gRPC NOT_GRPC_CAPABLE signal
    // must not flip it (otherwise JSON-RPC traffic loses the upstream too).
    let stats = harness.get_statistics("localnet").await?;
    let links = stats
        .links
        .ok_or_else(|| anyhow::anyhow!("no links in statistics response"))?;
    let mock_0 = links
        .iter()
        .find(|l| l.alias == "mock-0")
        .ok_or_else(|| anyhow::anyhow!("mock-0 not in links response"))?;
    assert!(
        !mock_0.status.to_lowercase().contains("down")
            && !mock_0.health_pct.contains("-100"),
        "mock-0 must NOT be marked DOWN after gRPC NOT_GRPC_CAPABLE (JSON-RPC should still see it as healthy); got status='{}' health_pct='{}'",
        mock_0.status,
        mock_0.health_pct
    );

    // At least one other mock-N must be healthy and serving gRPC.
    let any_other_ok = links.iter().any(|l| {
        l.alias.starts_with("mock-")
            && l.alias != "mock-0"
            && (l.status.to_lowercase().contains("ok") || l.health_pct.contains("+"))
    });
    assert!(
        any_other_ok,
        "expected at least one other mock-N to be healthy after mock-0 was demoted. links: {:#?}",
        links
    );

    Ok(())
}
