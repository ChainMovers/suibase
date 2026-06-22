#!/bin/bash

# ON-DEMAND wire-protocol parity: compare the local sb-local aggregator/publisher against a
# REAL Walrus aggregator/publisher (e.g. Suiftly's) on the HTTP contract a drop-in client
# relies on — ESPECIALLY error responses (bad params / missing blob / malformed id).
#
# Blobs differ across networks (localnet vs testnet/mainnet), so this compares HTTP STATUS
# CODES and RESPONSE SHAPES, not blob bytes. It is the cross-check that sb-local behaves like
# a production aggregator/publisher.
#
# Configure via env (each section is skipped if its URL is unset):
#   SB_LOCAL_BASE            local sb-local base   (default http://localhost:45840)
#   REAL_AGGREGATOR_URL      real aggregator base  (e.g. https://<suiftly-aggregator>)
#   REAL_PUBLISHER_URL       real publisher base   (e.g. https://<suiftly-publisher>)
#   REAL_PUBLISHER_AUTH      optional header for the real publisher, e.g.
#                            "Authorization: Bearer <token>"
#   RUN_PUBLISHER_WRITES=1   actually PUT to the REAL publisher (costs funds!) — off by default;
#                            without it, only NON-mutating bad-request cases hit the real publisher.
#
# Exit non-zero only on a real parity MISMATCH. Missing config => informational skip (exit 0).

set -uo pipefail

SB_LOCAL_BASE="${SB_LOCAL_BASE:-http://localhost:45840}"
_fail=0

c_status() { # url [extra curl args...] -> prints HTTP status code
  local url="$1"; shift
  curl -x "" -s -o /dev/null -w '%{http_code}' -m 20 "$@" "$url" 2>/dev/null
}

note() { echo "  $*"; }
ok() { echo "  ok  : $*"; }
bad() { echo "  FAIL: $*" >&2; _fail=1; }

# Compare two status codes for the "same" request against local vs real.
cmp_status() { # label local_code real_code [acceptable-set regex]
  local label="$1" lc="$2" rc="$3" accept="${4:-}"
  if [ -n "$accept" ]; then
    if [[ "$lc" =~ $accept ]] && [[ "$rc" =~ $accept ]]; then
      ok "$label: local=$lc real=$rc (both match /$accept/)"
    else
      bad "$label: local=$lc real=$rc (expected both /$accept/)"
    fi
  elif [ "$lc" = "$rc" ]; then
    ok "$label: local=$lc real=$rc (match)"
  else
    bad "$label: local=$lc real=$rc (mismatch)"
  fi
}

# A valid-format but never-stored blob id, and a deliberately malformed one.
MISSING_ID="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
BAD_ID="not-a-valid-blob-id"

echo "== sb-local reachable? =="
if ! curl -x "" -s -m 3 "$SB_LOCAL_BASE/status" | grep -q "OK"; then
  echo "SKIP: sb-local not reachable at $SB_LOCAL_BASE (start localnet with walrus enabled)"
  exit 0
fi
ok "sb-local up at $SB_LOCAL_BASE"

# ---------------- Aggregator (GET) error parity ----------------
if [ -n "${REAL_AGGREGATOR_URL:-}" ]; then
  echo "== aggregator error parity (sb-local vs $REAL_AGGREGATOR_URL) =="

  l=$(c_status "$SB_LOCAL_BASE/v1/blobs/$MISSING_ID")
  r=$(c_status "$REAL_AGGREGATOR_URL/v1/blobs/$MISSING_ID")
  cmp_status "GET nonexistent blob" "$l" "$r"

  l=$(c_status "$SB_LOCAL_BASE/v1/blobs/$BAD_ID")
  r=$(c_status "$REAL_AGGREGATOR_URL/v1/blobs/$BAD_ID")
  cmp_status "GET malformed blob id" "$l" "$r"

  l=$(c_status "$SB_LOCAL_BASE/v1/blobs/by-quilt-patch-id/$BAD_ID")
  r=$(c_status "$REAL_AGGREGATOR_URL/v1/blobs/by-quilt-patch-id/$BAD_ID")
  cmp_status "GET malformed quilt-patch id" "$l" "$r"
else
  echo "SKIP aggregator parity: set REAL_AGGREGATOR_URL (e.g. the Suiftly aggregator) to enable"
fi

# ---------------- Publisher (PUT) error parity ----------------
if [ -n "${REAL_PUBLISHER_URL:-}" ]; then
  echo "== publisher error parity (sb-local vs $REAL_PUBLISHER_URL) =="
  AUTH=()
  [ -n "${REAL_PUBLISHER_AUTH:-}" ] && AUTH=(-H "$REAL_PUBLISHER_AUTH")

  # Bad param: epochs=0 is invalid for a real publisher; sb-local should reject it too.
  # epochs=0: both must REJECT, but the exact code legitimately differs — the real publisher
  # returns 500 (its own quirk) while sb-local cleanly returns 400. Assert "both reject".
  l=$(c_status "$SB_LOCAL_BASE/v1/blobs?epochs=0" -X PUT --data-binary "parity-bad-epochs")
  r=$(c_status "$REAL_PUBLISHER_URL/v1/blobs?epochs=0" "${AUTH[@]}" -X PUT --data-binary "parity-bad-epochs")
  if [[ "$l" =~ ^[45][0-9][0-9]$ ]] && [[ "$r" =~ ^[45][0-9][0-9]$ ]]; then
    ok "PUT epochs=0: both reject (local=$l real=$r) [real 500s; sb-local cleanly 400s]"
  else
    bad "PUT epochs=0: both should reject (local=$l real=$r)"
  fi

  # Bad param: non-numeric epochs.
  l=$(c_status "$SB_LOCAL_BASE/v1/blobs?epochs=abc" -X PUT --data-binary "parity-bad-epochs2")
  r=$(c_status "$REAL_PUBLISHER_URL/v1/blobs?epochs=abc" "${AUTH[@]}" -X PUT --data-binary "parity-bad-epochs2")
  cmp_status "PUT epochs=abc" "$l" "$r"

  if [ "${RUN_PUBLISHER_WRITES:-0}" = "1" ]; then
    echo "== publisher success-shape parity (REAL writes enabled — costs funds) =="
    BODY="sb-local wire parity $(date +%s 2>/dev/null || echo fixed)"
    lj=$(curl -x "" -s -m 30 -X PUT --data-binary "$BODY" "$SB_LOCAL_BASE/v1/blobs?epochs=3")
    rj=$(curl -x "" -s -m 60 "${AUTH[@]}" -X PUT --data-binary "$BODY" "$REAL_PUBLISHER_URL/v1/blobs?epochs=3")
    for j in "$lj" "$rj"; do :; done
    # Both responses must carry a blobId under newlyCreated or alreadyCertified.
    extract_id() { echo "$1" | grep -oE '"blobId"[: ]*"[A-Za-z0-9_-]+"' | head -1 | grep -oE '[A-Za-z0-9_-]+"$' | tr -d '"'; }
    lid=$(extract_id "$lj"); rid=$(extract_id "$rj")
    if [ -n "$lid" ] && [ -n "$rid" ]; then
      ok "PUT success shape: both responses carry a blobId (local=$lid real=$rid)"
      # Same content => same canonical blob_id across both publishers (cross-environment).
      if [ "$lid" = "$rid" ]; then
        ok "same content -> same blob_id across sb-local and the real publisher"
      else
        note "blob_id differs (local=$lid real=$rid) — expected only if n_shards/encoding differ between networks"
      fi
    else
      bad "PUT success shape: missing blobId (local='$lj' real='$rj')"
    fi
  else
    note "RUN_PUBLISHER_WRITES not set — skipping real-publisher writes (set =1 to compare success shape; costs funds)"
  fi
else
  echo "SKIP publisher parity: set REAL_PUBLISHER_URL (+ REAL_PUBLISHER_AUTH if needed) to enable"
fi

echo
if [ "$_fail" -eq 0 ]; then
  echo "PASS: sb-local wire parity (no mismatches)"
  exit 0
else
  echo "FAIL: sb-local wire parity mismatches above" >&2
  exit 1
fi
