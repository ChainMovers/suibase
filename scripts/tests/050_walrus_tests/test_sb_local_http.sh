#!/bin/bash

# Live end-to-end test of sb-local: the localnet-only HTTP server exposing the Walrus
# aggregator + publisher wire API (GET/PUT /v1/blobs, quilts), backed by the nodeless
# LocalnetMockStore. Exercises the DROP-IN contract a real Walrus client relies on.
#
# Self-skipping: if sb-local is not reachable (no walrus-enabled localnet running), the
# test SKIPS (exit 2) so it is safe in the fast scripts-tests suite. The heavy
# walrus-localnet-integration.yml CI starts a walrus-enabled localnet first, so there
# sb-local IS up and the full round-trip runs. Set SB_LOCAL_HTTP_TEST=1 to turn a
# "not reachable" skip into a hard failure (used by that CI to assert it really ran).
#
# Coverage:
#   - GET /status liveness
#   - PUT /v1/blobs -> wire BlobStoreResult (newlyCreated) -> GET round-trip (bytes match)
#   - REAL blob id: cross-check against `walrus blob-id --n-shards 1000` (cross-env equality)
#   - HTTP Range -> 206 + Content-Range
#   - re-PUT identical bytes -> alreadyCertified (content dedup)
#   - GET unknown blob id -> 404
#   - Rust/HTTP interop: the PUT blob's bytes file exists in the shared blob dir
#   - Quilt: PUT /v1/quilts (multipart) -> read by-quilt-patch-id + by-quilt-id/identifier
#     -> GET /v1/quilts/{id}/patches
#
# See docs/dev/SB_LOCAL_PLAN.md and docs/dev/LOCALNET_WALRUS_FEATURE.md.

# Ignore SIGPIPE on macOS (consistent with the other 050 tests).
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
fi

SUIBASE_DIR="$HOME/suibase"
WORKDIRS="$SUIBASE_DIR/workdirs"

# Resolve the sb-local URL (its own independent bind/port). Allow override via env.
_PORT=$(sed -n 's/^sb_local_walrus_port:[[:space:]]*//p' "$WORKDIRS/localnet/suibase.yaml" 2>/dev/null | head -1)
_BIND=$(sed -n 's/^sb_local_host_ip:[[:space:]]*"\{0,1\}\([^"]*\)"\{0,1\}.*/\1/p' "$WORKDIRS/localnet/suibase.yaml" 2>/dev/null | head -1)
[ -n "$_PORT" ] || _PORT=45840
[ -n "$_BIND" ] || _BIND="127.0.0.1"
BASE="${SB_LOCAL_URL:-http://$_BIND:$_PORT}"

# curl wrapper that bypasses any http(s)_proxy (matches the faucet/relay health checks).
cget() { curl -x "" -s "$@"; }

skip() {
    if [ "${SB_LOCAL_HTTP_TEST:-}" = "1" ]; then
        echo "FAIL (SB_LOCAL_HTTP_TEST=1): $*" >&2
        exit 1
    fi
    echo "SKIP: $*"
    exit 2
}

fail() {
    echo "  FAIL: $*" >&2
    _fail=1
}

ok() { echo "  ok  : $*"; }

_fail=0

echo "=== sb-local HTTP wire test ($BASE) ==="

command -v python3 >/dev/null 2>&1 || skip "python3 not available (needed to parse JSON)"

# Liveness gate: skip cleanly if sb-local is not up (no walrus-enabled localnet).
if ! cget -m 3 "$BASE/status" | grep -q "OK"; then
    skip "sb-local not reachable at $BASE (start a walrus_local_enabled localnet first)"
fi
ok "/status -> OK"

# --- plain blob round-trip -------------------------------------------------
PAYLOAD="sb-local http test payload $(date +%s%N) $$"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
printf '%s' "$PAYLOAD" >"$TMP/payload.bin"

PUT_RESP=$(cget -X PUT --data-binary @"$TMP/payload.bin" "$BASE/v1/blobs?epochs=3")
BLOBID=$(printf '%s' "$PUT_RESP" | python3 -c '
import sys, json
d = json.load(sys.stdin)
nc = d.get("newlyCreated")
ac = d.get("alreadyCertified")
print(nc["blobObject"]["blobId"] if nc else (ac["blobId"] if ac else ""))
' 2>/dev/null)

if [ -n "$BLOBID" ]; then
    ok "PUT /v1/blobs -> blobId=$BLOBID"
else
    fail "PUT /v1/blobs did not return a blobId. Response: $PUT_RESP"
fi

# wire shape: newlyCreated must carry the camelCase blobObject fields.
if printf '%s' "$PUT_RESP" | python3 -c '
import sys, json
d = json.load(sys.stdin)
nc = d.get("newlyCreated") or {}
bo = nc.get("blobObject", {})
ro = nc.get("resourceOperation", {})
assert "blobId" in bo and "id" in bo and "storage" in bo and "certifiedEpoch" in bo, "blobObject shape"
assert "registerFromScratch" in ro, "resourceOperation shape"
' 2>/dev/null; then
    ok "PUT response matches the BlobStoreResult::newlyCreated wire shape"
else
    # Could be alreadyCertified if a prior run stored identical bytes (unlikely: nonce).
    printf '%s' "$PUT_RESP" | python3 -c 'import sys,json; assert "alreadyCertified" in json.load(sys.stdin)' 2>/dev/null \
        && ok "PUT response is alreadyCertified (dedup from a prior identical run)" \
        || fail "PUT response shape unexpected: $PUT_RESP"
fi

if [ -n "$BLOBID" ]; then
    # GET round-trip + headers.
    cget -D "$TMP/hdrs.txt" "$BASE/v1/blobs/$BLOBID" -o "$TMP/got.bin"
    if cmp -s "$TMP/payload.bin" "$TMP/got.bin"; then
        ok "GET /v1/blobs/$BLOBID round-trip bytes match"
    else
        fail "GET round-trip bytes mismatch"
    fi
    grep -qi "^etag:[[:space:]]*$BLOBID" "$TMP/hdrs.txt" && ok "ETag = blob id" || fail "ETag header missing/incorrect"
    grep -qi "^x-content-type-options:[[:space:]]*nosniff" "$TMP/hdrs.txt" && ok "X-Content-Type-Options: nosniff" || fail "nosniff header missing"
    grep -qi "^cache-control:" "$TMP/hdrs.txt" && ok "Cache-Control present" || fail "Cache-Control header missing"

    # Range -> 206.
    RCODE=$(cget -o "$TMP/range.bin" -w "%{http_code}" -H "Range: bytes=0-3" "$BASE/v1/blobs/$BLOBID")
    if [ "$RCODE" = "206" ] && [ "$(cat "$TMP/range.bin")" = "${PAYLOAD:0:4}" ]; then
        ok "Range bytes=0-3 -> 206 + correct slice"
    else
        fail "Range request expected 206 + 4-byte slice (got code=$RCODE body='$(cat "$TMP/range.bin")')"
    fi

    # re-PUT identical bytes -> alreadyCertified.
    RE_RESP=$(cget -X PUT --data-binary @"$TMP/payload.bin" "$BASE/v1/blobs?epochs=3")
    if printf '%s' "$RE_RESP" | python3 -c 'import sys,json; d=json.load(sys.stdin); ac=d["alreadyCertified"]; assert ac["blobId"] and ac["endEpoch"] and ac["object"]' 2>/dev/null; then
        ok "re-PUT identical bytes -> alreadyCertified (dedup)"
    else
        fail "re-PUT expected alreadyCertified. Response: $RE_RESP"
    fi

    # Rust/HTTP interop: bytes are content-addressed in the shared dir the Rust API uses.
    HEXKEY=$(printf '%s' "$BLOBID" | python3 -c '
import sys, base64
s = sys.stdin.read().strip()
pad = "=" * (-len(s) % 4)
print(base64.urlsafe_b64decode(s + pad).hex())
' 2>/dev/null)
    if [ -n "$HEXKEY" ] && [ -f "$WORKDIRS/localnet/config/walrus-localnet-blobs/$HEXKEY.bin" ]; then
        ok "interop: blob bytes present in the shared dir (Rust WalrusLocalClient reads the same file)"
    else
        fail "interop: shared-dir bytes file not found for $BLOBID"
    fi

    # Cross-environment REAL blob id: compare against the official walrus CLI.
    WBIN=$(ls "$WORKDIRS"/mainnet/bin/walrus "$WORKDIRS"/testnet/bin/walrus 2>/dev/null | head -1)
    if [ -n "$WBIN" ]; then
        WID=$("$WBIN" blob-id --n-shards 1000 "$TMP/payload.bin" 2>/dev/null | grep -oE 'Blob ID: [A-Za-z0-9_-]+' | awk '{print $3}')
        if [ -n "$WID" ]; then
            [ "$WID" = "$BLOBID" ] && ok "REAL id equality: walrus CLI computes the same blob id" \
                || fail "blob id mismatch vs walrus CLI ($WID != $BLOBID)"
        fi
    fi
fi

# 404 for an unknown (but well-formed) blob id.
NF=$(cget -o /dev/null -w "%{http_code}" "$BASE/v1/blobs/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
[ "$NF" = "404" ] && ok "GET unknown blob id -> 404" || fail "expected 404 for unknown blob id (got $NF)"

# --- quilt round-trip ------------------------------------------------------
printf 'alpha contents %s' "$(date +%s%N)" >"$TMP/alpha.txt"
printf 'beta DIFFERENT contents here' >"$TMP/beta.txt"
Q_RESP=$(cget -X PUT "$BASE/v1/quilts?epochs=2" \
    -F "alpha=@$TMP/alpha.txt" \
    -F "beta=@$TMP/beta.txt" \
    -F '_metadata=[{"identifier":"alpha","tags":{"kind":"text"}}]')

QUILTID=$(printf '%s' "$Q_RESP" | python3 -c '
import sys, json
d = json.load(sys.stdin)
b = d["blobStoreResult"]
nc = b.get("newlyCreated"); ac = b.get("alreadyCertified")
print(nc["blobObject"]["blobId"] if nc else (ac["blobId"] if ac else ""))
' 2>/dev/null)
PATCH_ALPHA=$(printf '%s' "$Q_RESP" | python3 -c '
import sys, json
d = json.load(sys.stdin)
print(next(p["quiltPatchId"] for p in d["storedQuiltBlobs"] if p["identifier"] == "alpha"))
' 2>/dev/null)

if [ -n "$QUILTID" ] && [ -n "$PATCH_ALPHA" ]; then
    ok "PUT /v1/quilts -> quiltId=$QUILTID (alpha patch id parsed)"

    cget -D "$TMP/qhdr.txt" "$BASE/v1/blobs/by-quilt-patch-id/$PATCH_ALPHA" -o "$TMP/qa.txt"
    cmp -s "$TMP/alpha.txt" "$TMP/qa.txt" && ok "by-quilt-patch-id alpha bytes match" || fail "by-quilt-patch-id alpha mismatch"
    grep -qi "^x-quilt-patch-identifier:[[:space:]]*alpha" "$TMP/qhdr.txt" && ok "X-Quilt-Patch-Identifier header set" || fail "X-Quilt-Patch-Identifier missing"

    cget "$BASE/v1/blobs/by-quilt-id/$QUILTID/beta" -o "$TMP/qb.txt"
    cmp -s "$TMP/beta.txt" "$TMP/qb.txt" && ok "by-quilt-id/{id}/beta bytes match" || fail "by-quilt-id beta mismatch"

    LIST=$(cget "$BASE/v1/quilts/$QUILTID/patches")
    if printf '%s' "$LIST" | python3 -c '
import sys, json
items = json.load(sys.stdin)
ids = {i["identifier"] for i in items}
assert {"alpha", "beta"} <= ids, ids
assert all("patchId" in i and "tags" in i for i in items)
alpha = next(i for i in items if i["identifier"] == "alpha")
assert alpha["tags"].get("kind") == "text", alpha["tags"]
' 2>/dev/null; then
        ok "GET /v1/quilts/{id}/patches lists alpha+beta with tags"
    else
        fail "patches list shape unexpected: $LIST"
    fi
else
    fail "quilt PUT did not return quiltId + patch id. Response: $Q_RESP"
fi

echo
if [ "$_fail" -eq 0 ]; then
    echo "PASS: sb-local HTTP wire test"
    exit 0
else
    echo "FAIL: sb-local HTTP wire test" >&2
    exit 1
fi
