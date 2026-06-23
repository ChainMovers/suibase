#!/bin/bash

# lwalrus <-> walrus parity test (FUND-FREE, ALWAYS-ON).
#
# lwalrus is a SUBSET of the real `walrus` CLI (see docs/dev/LWALRUS_LSITE_PLAN.md).
# The bar is NOT a byte-exact --help clone (different command set; the localnet stack
# is pinned to a possibly-different walrus rev than the shipped binary). Instead this
# test is SEMANTIC: it compares only the SUPPORTED command surface against the real
# `walrus` and IGNORES everything lwalrus explicitly lists under "Not supported for
# localnet:". It flags drift among the supported commands and, crucially, fails when
# `walrus` grows a command that lwalrus neither supports nor lists as unsupported
# (so we triage it: implement, or add to the unsupported list).
#
# Everything here is fund-free and runs without a wallet. The optional funded
# store/read/delete round-trip at the end is skipped unless a localnet with
# walrus_local_enabled is up.
#
# Exit codes: 0 pass, 1 fail, 2 skip.

__ORIGINAL_DIR=$PWD
cd "$(dirname "${BASH_SOURCE[0]}")" || exit 1

# Self-contained on purpose: this is a read-only, fund-free parity check, so it does
# NOT source __test_common.sh (which restores workdir config at source time). It is
# independent of the harness's CI_WORKDIR.
SUIBASE_DIR="$HOME/suibase"
WORKDIRS="$SUIBASE_DIR/workdirs"
fail() { echo "ERROR: $*" >&2; exit 1; }

echo "=== lwalrus <-> walrus parity (fund-free) ==="

# ---- Resolve binaries (cross-workdir; the binaries live in different workdirs) ----
resolve_lwalrus() {
  local c
  for c in "$WORKDIRS/common/bin/lwalrus" \
           "$SUIBASE_DIR/rust/localnet-tools/target/release/lwalrus" \
           "$SUIBASE_DIR/rust/localnet-tools/target/debug/lwalrus"; do
    [ -x "$c" ] && { echo "$c"; return 0; }
  done
  return 1
}
resolve_walrus() {
  local c
  for c in "$WORKDIRS/testnet/bin/walrus" "$WORKDIRS/mainnet/bin/walrus"; do
    [ -x "$c" ] && { echo "$c"; return 0; }
  done
  return 1
}

LW="$(resolve_lwalrus)" || { echo "SKIP: lwalrus binary not built (run 'localnet update' or build rust/localnet-tools)."; exit 2; }
echo "lwalrus: $LW"

# ============================================================================
# Section A — lwalrus self-checks (always; no walrus, no funds, no localnet)
# ============================================================================
echo "--- A: lwalrus self-checks ---"

# A1: help advertises the explicit unsupported sections (commands + options).
"$LW" --help 2>&1 | grep -q "Not supported for localnet (commands):" || fail "lwalrus --help is missing the 'Not supported for localnet (commands):' section"
"$LW" --help 2>&1 | grep -q "Not supported for localnet (options):" || fail "lwalrus --help is missing the 'Not supported for localnet (options):' section"
echo "  A1 ok: --help has both 'Not supported for localnet' (commands + options) sections"

# A2: an unsupported command prints a clear message and exits non-zero.
"$LW" stake 100 >/tmp/lwp_o 2>&1; rc=$?
[ "$rc" -ne 0 ] || fail "lwalrus stake should exit non-zero (got 0)"
grep -q "Not supported for localnet" /tmp/lwp_o || fail "lwalrus stake should print 'Not supported for localnet' (got: $(cat /tmp/lwp_o))"
echo "  A2 ok: unsupported command -> 'Not supported for localnet' (exit $rc)"

# A3: a malformed blob id is rejected at the parse layer with exit 2 (matches walrus).
"$LW" read not-a-real-id >/tmp/lwp_o 2>&1; rc=$?
[ "$rc" -eq 2 ] || fail "lwalrus read <bad-id> should exit 2 (clap parse), got $rc"
grep -q "invalid value 'not-a-real-id' for '<BLOB_ID>'" /tmp/lwp_o || fail "lwalrus bad-id message differs from walrus: $(cat /tmp/lwp_o)"
echo "  A3 ok: malformed blob id -> exit 2 + walrus-matching message"

# A4: a missing required argument is a clap usage error (exit 2).
"$LW" store >/tmp/lwp_o 2>&1; rc=$?
[ "$rc" -eq 2 ] || fail "lwalrus store (missing FILE) should exit 2, got $rc"
echo "  A4 ok: missing required arg -> exit 2"

# A5: an unsupported global option (e.g. --config) is rejected with the clear message.
# (Provide a FILE so the parse succeeds and the option check is what fires.)
printf 'x' > /tmp/lwp_cfg.txt
"$LW" store --config /tmp/whatever /tmp/lwp_cfg.txt >/tmp/lwp_o 2>&1; rc=$?
[ "$rc" -ne 0 ] || fail "lwalrus store --config should be rejected (got exit 0)"
grep -q "Not supported for localnet" /tmp/lwp_o || fail "lwalrus --config should print 'Not supported for localnet' (got: $(cat /tmp/lwp_o))"
echo "  A5 ok: unsupported global option (--config) -> 'Not supported for localnet'"

# ============================================================================
# Section B — semantic surface parity vs the real walrus (when available)
# ============================================================================
WAL="$(resolve_walrus)"
if [ -z "$WAL" ]; then
  echo "--- B: SKIPPED (no shipped 'walrus' binary found in testnet/mainnet bin) ---"
else
  echo "--- B: semantic surface parity vs $WAL ($("$WAL" --version 2>/dev/null | tail -1)) ---"

  # Authoritative command list of a binary: take candidate tokens from the Commands:
  # block, then keep only those for which `<bin> <tok> --help` succeeds (filters out
  # wrapped-description noise). Fund-free (only --help is invoked).
  real_commands() {
    local bin="$1" tok
    "$bin" --help 2>&1 \
      | awk '/^Commands:/{f=1;next} /^[A-Za-z].*:$/{if(f)f=0} f' \
      | awk '{print $1}' | grep -E '^[a-z][a-z0-9-]*$' | sort -u \
      | while read -r tok; do
          if "$bin" "$tok" --help >/dev/null 2>&1; then echo "$tok"; fi
        done
  }

  mapfile -t WAL_CMDS < <(real_commands "$WAL")
  mapfile -t LW_SUPPORTED < <(real_commands "$LW")
  # lwalrus's explicit unsupported list: the 4-space-indented name lines under the
  # "Not supported for localnet:" section (prose headers are 2-space + capitalized).
  # Only the COMMANDS sub-section (stop at the options sub-section). Option lines start
  # with '--' so they would not match '^    [a-z]' anyway, but bounding is clearer.
  mapfile -t LW_UNSUPPORTED < <("$LW" --help 2>&1 \
    | awk '/^Not supported for localnet \(commands\):/{f=1;next} /^Not supported for localnet \(options\):/{f=0} f' \
    | grep -E '^    [a-z]' | tr ',' ' ' | tr -s ' ' '\n' | grep -E '^[a-z][a-z0-9-]*$' | sort -u)

  echo "  walrus commands: ${#WAL_CMDS[@]} | lwalrus supported: ${LW_SUPPORTED[*]} | unsupported-listed: ${#LW_UNSUPPORTED[@]}"

  in_list() { local x="$1"; shift; local e; for e in "$@"; do [ "$e" = "$x" ] && return 0; done; return 1; }

  # Meta/builtin commands present on any clap app — not a parity concern.
  META="help options completion"

  # B1: every lwalrus SUPPORTED command must exist in walrus (no invented commands).
  for c in "${LW_SUPPORTED[@]}"; do
    in_list "$c" $META && continue
    in_list "$c" "${WAL_CMDS[@]}" || fail "lwalrus supports '$c' which the real walrus does not have (renamed/removed upstream?)"
  done
  echo "  B1 ok: every lwalrus-supported command exists in walrus"

  # B2 (drift detector): every walrus command must be accounted for — supported by
  # lwalrus, OR in lwalrus's unsupported list, OR a clap meta command. A walrus command
  # in none of these is NEW upstream and must be triaged.
  unaccounted=()
  for c in "${WAL_CMDS[@]}"; do
    in_list "$c" $META && continue
    in_list "$c" "${LW_SUPPORTED[@]}" && continue
    in_list "$c" "${LW_UNSUPPORTED[@]}" && continue
    unaccounted+=("$c")
  done
  if [ "${#unaccounted[@]}" -ne 0 ]; then
    fail "walrus has command(s) lwalrus neither supports nor lists as unsupported: ${unaccounted[*]}
  -> Triage: implement in lwalrus, or add to NOT_SUPPORTED_HELP in rust/localnet-tools/src/bin/lwalrus/main.rs"
  fi
  echo "  B2 ok: every walrus command is accounted for (supported or explicitly unsupported)"

  # B3: failure-mode parity — both reject a malformed blob id at parse time (exit 2).
  "$WAL" read not-a-real-id >/tmp/lwp_w 2>&1; wrc=$?
  "$LW"  read not-a-real-id >/tmp/lwp_l 2>&1; lrc=$?
  [ "$wrc" -eq "$lrc" ] || fail "bad-blob-id exit code differs: walrus=$wrc lwalrus=$lrc"
  echo "  B3 ok: malformed blob id -> both exit $wrc"
fi

# ============================================================================
# Section C — funded round-trip (optional; skipped unless localnet+walrus is up)
# ============================================================================
echo "--- C: funded round-trip (optional) ---"
localnet_walrus_up() {
  grep -qE "^walrus_local_enabled:[[:space:]]*true" "$WORKDIRS/localnet/suibase.yaml" 2>/dev/null || return 1
  [ -f "$WORKDIRS/localnet/config-default/walrus-localnet.yaml" ] || return 1
  curl -fsS -m 3 -X POST http://localhost:9000 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"sui_getChainIdentifier","params":[]}' >/dev/null 2>&1
}
if ! localnet_walrus_up; then
  echo "  SKIPPED: localnet not up / walrus_local_enabled not set / not deployed."
else
  printf 'lwalrus parity %s\n' "$(date -u +%s 2>/dev/null || echo now)" > /tmp/lwp_in.txt
  "$LW" store /tmp/lwp_in.txt --epochs 1 >/tmp/lwp_store 2>&1 || fail "round-trip store failed: $(cat /tmp/lwp_store)"
  bid="$(sed -n 's/.*blob id:[[:space:]]*//p' /tmp/lwp_store | head -1)"
  [ -n "$bid" ] || fail "round-trip: could not capture blob id from store output"
  "$LW" read "$bid" --out /tmp/lwp_out.txt >/dev/null 2>&1 || fail "round-trip read failed for $bid"
  cmp -s /tmp/lwp_in.txt /tmp/lwp_out.txt || fail "round-trip: read bytes differ from stored bytes"
  "$LW" blob-status "$bid" >/dev/null 2>&1 || fail "round-trip blob-status failed for $bid"
  "$LW" delete "$bid" >/dev/null 2>&1 || fail "round-trip delete failed for $bid"
  echo "  C ok: store -> read (bytes match) -> blob-status -> delete round-trip"
fi

echo "=== lwalrus parity test passed ==="
cd "$__ORIGINAL_DIR" || true
exit 0
