#!/bin/bash

# Test for RPC proxy response integrity, on the gRPC path.
#
# Originally this drove JSON-RPC (sui_getChainIdentifier and friends) through
# the proxy to prove the daemon hands back a decompressed response instead of
# raw gzip bytes. Sui has since deprecated JSON-RPC on public fullnodes -- they
# answer every method with -32601 "JSON-RPC on public fullnodes has been
# deprecated. Please migrate to gRPC or GraphQL endpoints" -- so the JSON-RPC
# form of this test can no longer reach a real upstream and asserts nothing.
#
# The underlying concern is unchanged on the gRPC path (see the daemon commit
# "proxy: retry gRPC upstreams returning gzip-compressed responses"), so the
# same guarantee is asserted here over gRPC: whatever the upstream sends and
# whatever encoding the client offers, the proxy must return a well-formed
# frame the client can actually decode. See validate_grpc_frame() for why that
# is not the same as "uncompressed" -- negotiated compression is legal.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the walrus test common utilities (which includes build daemon functionality)
. "$SCRIPT_DIR/__test_common.sh"

TEST_NAME="RPC Proxy Compression Fix (gRPC)"
TEST_DESCRIPTION="Verify suibase RPC proxy returns gRPC frames the client can decode"

# Test configuration
WORKDIR="testnet"
PROXY_PORT="44342"

# Unary, no-parameter gRPC method on the Sui LedgerService. The response
# carries chain id / epoch / checkpoint height -- small and cheap. This is the
# same call the daemon uses for its own upstream health probing.
GRPC_PATH="/sui.rpc.v2.LedgerService/GetServiceInfo"

# A gRPC request frame for an empty message:
#   byte 0   : compression flag (0 = uncompressed)
#   bytes 1-4: message length, big-endian (0)
EMPTY_GRPC_FRAME='\x00\x00\x00\x00\x00'

# The client-offered encodings to exercise. Offering gzip is the interesting
# case: the server may then legally compress, provided it says so.
declare -a TEST_ENCODINGS=(
    "identity"
    "gzip"
    "gzip,identity"
    "gzip,deflate"
    "deflate"
    "identity,gzip"
)

MAX_ATTEMPTS=${#TEST_ENCODINGS[@]}

# Scratch files, set up in setup_test_environment.
GRPC_REQ_FILE=""
GRPC_RESP_FILE=""
GRPC_HDR_FILE=""

# Issue one gRPC request through the proxy.
# Echoes the HTTP status code; writes the response body to $GRPC_RESP_FILE and
# the response headers to $GRPC_HDR_FILE (needed to tell a legitimately
# negotiated compressed message from a compressed one the client cannot decode).
grpc_call_via_proxy() {
    local accept_encoding="$1"
    local timeout="${2:-10}"

    curl -s --http2-prior-knowledge --max-time "$timeout" \
        -o "$GRPC_RESP_FILE" \
        -D "$GRPC_HDR_FILE" \
        -w "%{http_code}" \
        -X POST "http://localhost:$PROXY_PORT$GRPC_PATH" \
        -H "content-type: application/grpc" \
        -H "te: trailers" \
        -H "grpc-accept-encoding: $accept_encoding" \
        --data-binary "@$GRPC_REQ_FILE" 2>/dev/null
}

# Portable "first N bytes as lowercase hex, no spaces".
# Uses od (BSD + GNU) rather than xxd, which is not guaranteed on macOS.
head_bytes_hex() {
    local file="$1"
    local count="$2"
    od -An -tx1 -N "$count" "$file" 2>/dev/null | tr -d ' \n'
}

setup_test_environment() {
    echo "--- Setting up test environment"

    GRPC_REQ_FILE="$(mktemp)"
    GRPC_RESP_FILE="$(mktemp)"
    GRPC_HDR_FILE="$(mktemp)"
    # shellcheck disable=SC2059
    printf "$EMPTY_GRPC_FRAME" >"$GRPC_REQ_FILE"

    # Setup clean environment and ensure BUILD version of daemon
    setup_clean_environment
    setup_test_workdir "$WORKDIR"
    backup_config_files "$WORKDIR"

    # Ensure BUILD version of suibase-daemon is used (not precompiled)
    ensure_build_daemon

    # Start daemon using safe method that ensures BUILD version
    safe_start_daemon

    # Wait for daemon to be ready
    wait_for_daemon_running 15 true

    # Wait for the proxy to answer on the specific port.
    #
    # Probe with the real gRPC call rather than a bare GET. A GET is neither a
    # JSON-RPC nor a gRPC request, so the daemon forwards it to the configured
    # upstreams and it inherits their behaviour -- which today means it burns
    # the retry budget and takes ~11s to return a 500, long past any sane
    # readiness timeout, even while the proxy is perfectly healthy.
    local attempts=0
    while [ $attempts -lt 10 ]; do
        if [ "$(grpc_call_via_proxy "identity" 2)" = "200" ]; then
            echo "✓ RPC proxy is responding to gRPC"
            return 0
        fi
        ((attempts++))
        sleep 1
    done

    echo "✗ RPC proxy failed to start within timeout"
    return 1
}

# Validate a gRPC response frame written to $GRPC_RESP_FILE.
#   0 = frame the client can decode
#   2 = malformed, or compressed in a way the client cannot decode
#
# Note on what is and isn't a bug. Per the gRPC spec, when the client advertises
# an encoding via grpc-accept-encoding the server MAY compress the message: it
# sets the per-message compression flag to 1 AND announces the algorithm in a
# `grpc-encoding` response header, which is how the client knows to decompress.
# That is correct behaviour, not the bug -- an earlier version of this test
# asserted the flag was always 0, which merely encoded the behaviour of one
# daemon build that happens never to compress.
#
# The actual bug is compressed bytes the client cannot decode: a set flag with
# no `grpc-encoding` header, an algorithm the client never offered, or a raw
# compressed stream in place of a gRPC frame.
validate_grpc_frame() {
    local offered="$1"

    local size
    size=$(wc -c <"$GRPC_RESP_FILE" | tr -d ' ')

    # A gRPC frame is a 5-byte header plus payload.
    if [ "$size" -lt 5 ]; then
        echo "  ✗ response too short to be a gRPC frame ($size bytes)"
        return 2
    fi

    local header
    header=$(head_bytes_hex "$GRPC_RESP_FILE" 5)

    # gzip magic (1f 8b) at offset 0 means a raw compressed stream arrived
    # instead of a gRPC frame -- no framing at all, nothing can parse it.
    if [ "${header:0:4}" = "1f8b" ]; then
        echo "  ✗ CRITICAL: response starts with gzip magic (raw compressed stream, not a gRPC frame)"
        return 2
    fi

    # Bytes 1-4 are the big-endian payload length, and must agree with what
    # actually arrived -- true whether or not the payload is compressed.
    local declared_len=$((16#${header:2:8}))
    local actual_len=$((size - 5))
    if [ "$declared_len" -ne "$actual_len" ]; then
        echo "  ✗ CRITICAL: frame length mismatch (declared $declared_len, got $actual_len)"
        return 2
    fi

    # Byte 0 is the per-message compression flag.
    local flag="${header:0:2}"
    if [ "$flag" = "00" ]; then
        echo "  ✓ uncompressed frame (${actual_len}B payload)"
        return 0
    fi

    # Flag set: legitimate only if the algorithm was announced and the client
    # offered it.
    local announced
    announced=$(grep -i "^grpc-encoding:" "$GRPC_HDR_FILE" 2>/dev/null |
        head -1 | cut -d: -f2- | tr -d ' \r\n')

    if [ -z "$announced" ]; then
        echo "  ✗ CRITICAL: compression flag 0x$flag but no grpc-encoding header"
        echo "    the client is handed compressed bytes with no way to know the algorithm"
        return 2
    fi

    if ! echo ",$offered," | grep -qi ",$announced,"; then
        echo "  ✗ CRITICAL: response encoded with '$announced', which the client did not offer ($offered)"
        return 2
    fi

    echo "  ✓ compressed frame, properly negotiated (grpc-encoding: $announced, ${actual_len}B payload)"
    return 0
}

# Test a single gRPC query through the proxy.
test_grpc_query() {
    local accept_encoding="$1"
    local attempt="$2"

    echo "--- Test $attempt/$MAX_ATTEMPTS: grpc-accept-encoding: $accept_encoding"

    local http_code
    http_code=$(grpc_call_via_proxy "$accept_encoding" 10)
    local curl_exit_code=$?

    if [ $curl_exit_code -ne 0 ]; then
        echo "  ⚠ Query $attempt failed - curl error $curl_exit_code"
        return 1
    fi

    if [ "$http_code" != "200" ]; then
        echo "  ⚠ Query $attempt failed - HTTP $http_code"
        return 1
    fi

    validate_grpc_frame "$accept_encoding"
    local frame_result=$?
    if [ $frame_result -ne 0 ]; then
        return $frame_result
    fi

    echo "  ✓ Query $attempt passed - client-decodable gRPC frame"
    return 0
}

run_compression_tests() {
    echo "--- Running RPC proxy compression tests (gRPC)"

    local total_tests=${#TEST_ENCODINGS[@]}
    local passed_tests=0
    local failed_tests=0
    local compression_bugs=0

    for i in "${!TEST_ENCODINGS[@]}"; do
        local accept_encoding="${TEST_ENCODINGS[$i]}"
        local attempt=$((i + 1))

        test_grpc_query "$accept_encoding" "$attempt"
        local result=$?

        case $result in
        0)
            ((passed_tests++))
            ;;
        1)
            ((failed_tests++))
            ;;
        2)
            ((compression_bugs++))
            ;;
        esac

        # Small delay between tests
        sleep 0.5
    done

    echo "--- Test Results Summary"
    echo "  Total tests: $total_tests"
    echo "  Passed: $passed_tests"
    echo "  Failed (network/other): $failed_tests"
    echo "  Failed (compression bug): $compression_bugs"

    if [ $compression_bugs -gt 0 ]; then
        echo "✗ CRITICAL: RPC proxy compression bug detected!"
        echo "The suibase RPC proxy is returning compressed or malformed gRPC frames."
        echo "This breaks the sui CLI and other gRPC clients."
        echo "This test will FAIL to ensure the bug gets fixed."
        return 1
    fi

    if [ $passed_tests -eq 0 ]; then
        echo "✗ All tests failed - RPC proxy appears to be non-functional"
        return 1
    fi

    echo "✓ No compression bugs detected - all responses are client-decodable gRPC frames"
    return 0
}

cleanup_test_environment() {
    echo "--- Cleaning up test environment"

    # Restore config files
    restore_config_files "$WORKDIR"

    # Clean up any port conflicts (stop processes using our ports)
    cleanup_port_conflicts

    [ -n "$GRPC_REQ_FILE" ] && rm -f "$GRPC_REQ_FILE"
    [ -n "$GRPC_RESP_FILE" ] && rm -f "$GRPC_RESP_FILE"
    [ -n "$GRPC_HDR_FILE" ] && rm -f "$GRPC_HDR_FILE"

    return 0
}

main() {
    echo "================================"
    echo "$TEST_NAME"
    echo "================================"
    echo "Description: $TEST_DESCRIPTION"
    echo "Workdir: $WORKDIR"
    echo "Proxy Port: $PROXY_PORT"
    echo "gRPC Method: $GRPC_PATH"
    echo "Test Queries: ${#TEST_ENCODINGS[@]}"
    echo ""

    # Set up error handling
    trap cleanup_test_environment EXIT

    # Run the test sequence
    if ! setup_test_environment; then
        echo "✗ Test environment setup failed"
        exit 1
    fi

    if ! run_compression_tests; then
        echo "✗ $TEST_NAME FAILED"
        exit 1
    fi

    echo "✓ $TEST_NAME PASSED"
    exit 0
}

# Run the test if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
