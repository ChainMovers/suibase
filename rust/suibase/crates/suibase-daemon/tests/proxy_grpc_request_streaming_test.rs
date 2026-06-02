// Regression test for the gRPC *request*-stream pass-through — the companion
// to proxy_grpc_streaming_test.rs, which covers the *response* direction.
//
// A client/bidi-streaming gRPC client (e.g. gRPC server reflection — what
// grpcurl/grpcui/Postman use) keeps its request stream open: it sends a message
// and waits for responses before sending more or closing, so END_STREAM may not
// arrive for a long time (or ever, for a `list`). The proxy used to buffer the
// entire request body before forwarding and reject anything that didn't reach
// END_STREAM within 5s with UNIMPLEMENTED — which broke reflection and made
// `grpcurl` (and gRPC tooling generally) fail against the proxy.
//
// The proxy now classifies by method:
//   * `sui.rpc.v2.*` / `grpc.health.*`  → buffered + retry (unchanged)
//   * everything else (reflection, …)   → request body piped live to one
//                                          upstream, both directions concurrent
//
// Each mock answers gRPC *without reading the request body* (like a real bidi
// server), so a correctly-piping proxy returns a response promptly even though
// the client never closes its request stream. A proxy that buffers would hang
// until its internal timeout and blow the 3s budget below.

mod common;

use anyhow::{anyhow, Result};
use common::{clear_all_rate_limits, reset_all_mock_servers, MockServerTestHarness};
use std::time::Duration;
use tokio::time::sleep;

// Server reflection: bidi-streaming, NOT on the single-request whitelist, so the
// proxy must classify it as streaming by path and pipe immediately (0 latency).
const REFLECTION: &str = "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo";

// A whitelisted (`sui.rpc.v2.*`) method: normally buffered, but an open request
// body must still end up piped via the buffered-path safety net rather than
// hanging.
const WHITELISTED: &str = "/sui.rpc.v2.LedgerService/GetServiceInfo";

async fn ready_harness() -> Result<MockServerTestHarness> {
    let harness = MockServerTestHarness::new().await?;
    reset_all_mock_servers(&harness).await?;
    clear_all_rate_limits(&harness).await?;
    sleep(Duration::from_secs(2)).await;
    harness
        .ensure_servers_healthy(&["mock-0", "mock-1", "mock-2", "mock-3", "mock-4"])
        .await?;
    Ok(harness)
}

fn assert_grpc_ok(response: &reqwest::Response) {
    assert_eq!(
        response.status().as_u16(),
        200,
        "expected HTTP 200 for a piped request stream, got {}",
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
    // The mock answers a piped request with grpc-status 0 (OK). The pre-fix
    // proxy answered an open request stream with grpc-status 12 (UNIMPLEMENTED)
    // — assert OK so the test fails loudly on that regression even if the
    // rejection somehow arrived within the time budget. (Real upstreams deliver
    // grpc-status in trailers, absent from headers; the check only binds when
    // present, as it is for the mock's header-only response.)
    if let Some(status) = response
        .headers()
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
    {
        assert_eq!(
            status, "0",
            "expected grpc-status 0 (OK); got {} — the request was rejected \
             (e.g. UNIMPLEMENTED) instead of piped through",
            status
        );
    }
}

// Reflection (or any non-whitelisted method) with a never-closing request
// stream must be piped through and answered, NOT buffered-then-timed-out.
#[tokio::test]
async fn test_proxy_pipes_open_request_stream_for_reflection() -> Result<()> {
    let harness = ready_harness().await?;

    let response = tokio::time::timeout(
        Duration::from_secs(3),
        harness.send_grpc_request_open_body(REFLECTION),
    )
    .await
    .map_err(|_| {
        anyhow!(
            "Proxy did not respond to an open (never-closing) reflection request \
             stream within 3s. The request body is being buffered instead of \
             piped through — bidi/client-streaming (incl. server reflection) is \
             broken."
        )
    })??;

    assert_grpc_ok(&response);

    // The request stream is infinite; dropping the response cancels it.
    drop(response);
    Ok(())
}

// A whitelisted method normally takes the buffered (retry) path, but if its
// request body stays open the grace-window safety net must switch to piping
// rather than hang. (No Sui method ships a streaming request today; this guards
// the case where one is added under the whitelisted prefixes.)
#[tokio::test]
async fn test_proxy_safety_net_pipes_open_whitelisted_request() -> Result<()> {
    let harness = ready_harness().await?;

    let response = tokio::time::timeout(
        Duration::from_secs(3),
        harness.send_grpc_request_open_body(WHITELISTED),
    )
    .await
    .map_err(|_| {
        anyhow!(
            "Proxy did not respond to an open request stream on a whitelisted \
             method within 3s. The buffered-path safety net isn't falling back \
             to piping when END_STREAM never arrives."
        )
    })??;

    assert_grpc_ok(&response);
    drop(response);
    Ok(())
}
