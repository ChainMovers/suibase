// Regression test for the gRPC streaming pass-through.
//
// Sets up every mock to answer gRPC requests with a server-streaming response
// that never closes (emits an empty DATA frame every 100 ms, never sends
// END_STREAM). A correctly-implemented proxy must forward the response
// headers to the client immediately as they arrive from upstream, then pipe
// the body frames through. A proxy that calls BodyExt::collect on the
// response will hang waiting for END_STREAM, and the client won't see the
// response headers until the proxy's own UPSTREAM timeout fires (30 seconds).
//
// The test bounds the client-side wait at 3 seconds — well under the proxy's
// internal timeout. If the proxy buffers, the test fails (no response
// within budget). If it streams, the test passes (response headers arrive
// fast and have the expected status + content-type).

mod common;

use anyhow::{anyhow, Result};
use common::{
    clear_all_rate_limits, grpc_streaming_behavior, reset_all_mock_servers, MockServerTestHarness,
};
use std::time::Duration;
use tokio::time::sleep;

const GET_SERVICE_INFO: &str = "/sui.rpc.v2.LedgerService/GetServiceInfo";

#[tokio::test]
async fn test_proxy_passes_streaming_response_through() -> Result<()> {
    let harness = MockServerTestHarness::new().await?;

    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    sleep(Duration::from_secs(2)).await;

    harness
        .ensure_servers_healthy(&["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"])
        .await?;

    // Configure every mock to answer gRPC with a never-ending streaming body.
    // No matter which upstream the proxy picks, it will face the streaming
    // case the bug needs to handle.
    for alias in ["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"] {
        harness
            .configure_mock_server(alias, grpc_streaming_behavior())
            .await?;
    }

    // Send one gRPC request; bound the wait at 3 seconds. reqwest's `.send()`
    // returns when response headers arrive (HTTP/2), so a working proxy must
    // produce a Response well under 3 s. A buffering proxy will keep us
    // waiting until its internal UPSTREAM_TIMEOUT (~30 s) fires.
    let response = tokio::time::timeout(
        Duration::from_secs(3),
        harness.send_grpc_request(GET_SERVICE_INFO),
    )
    .await
    .map_err(|_| {
        anyhow!(
            "Proxy did not forward response headers within 3s. Likely the \
             response body is being buffered (BodyExt::collect) instead of \
             streamed through to the client."
        )
    })??;

    assert_eq!(
        response.status().as_u16(),
        200,
        "expected HTTP 200 from streaming upstream, got {}",
        response.status()
    );

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(
        content_type.starts_with("application/grpc"),
        "expected content-type starting with 'application/grpc', got '{}'",
        content_type
    );

    // We deliberately don't read the body — it's an infinite stream. Dropping
    // `response` cancels the upstream stream on the proxy side.
    drop(response);

    Ok(())
}
