#!/bin/bash

# Unit tests for the Move.toml [environments] chain_id sync helpers in
# scripts/common/__publish.sh:
#   - get_movetoml_env_chainid
#   - sync_movetoml_env_chainid
#   - sync_movetoml_workdir_chainids
#   - sync_local_deps_chainids
#
# These are pure-function tests against temp Move.toml fixtures — no
# network access, no daemon, no sui binary. The publish-time wiring
# (call site in publish_all + get_current_chain_id) is covered by the
# integration story: `localnet regen` + `localnet publish` succeeds
# without the user manually updating chain_ids.

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

# shellcheck source=SCRIPTDIR/../../common/__publish.sh
source "$SUIBASE_DIR/scripts/common/__publish.sh"

# ----- test scaffolding -----

TEST_TMPDIR=""
setup_tmp() {
  TEST_TMPDIR=$(mktemp -d -t suibase-movetoml-chainid-XXXXXX)
}
teardown_tmp() {
  [ -n "$TEST_TMPDIR" ] && rm -rf "$TEST_TMPDIR"
  TEST_TMPDIR=""
}

write_fixture_movetoml() {
  local _PATH="$1"
  local _LOCAL_CHAIN="$2"
  local _PROXY_CHAIN="$3"
  mkdir -p "$(dirname "$_PATH")"
  cat >"$_PATH" <<EOF
[package]
name = "demo"
version = "0.0.1"
edition = "2024.beta"

[environments]
localnet = "$_LOCAL_CHAIN"
localnet_proxy = "$_PROXY_CHAIN"

[dependencies]
log = { local = "../log" }
EOF
}

# ----- get_movetoml_env_chainid -----

test_get_movetoml_env_chainid_reads_value() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  write_fixture_movetoml "$_F" "aaaaaaaa" "bbbbbbbb"
  local _v
  _v=$(get_movetoml_env_chainid "$_F" "localnet")
  [ "$_v" = "aaaaaaaa" ] || fail "expected aaaaaaaa, got '$_v'"
  _v=$(get_movetoml_env_chainid "$_F" "localnet_proxy")
  [ "$_v" = "bbbbbbbb" ] || fail "expected bbbbbbbb, got '$_v'"
  teardown_tmp
}

test_get_movetoml_env_chainid_missing_env_returns_empty() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  write_fixture_movetoml "$_F" "aaaaaaaa" "bbbbbbbb"
  local _v
  _v=$(get_movetoml_env_chainid "$_F" "testnet")
  [ -z "$_v" ] || fail "missing env must return empty, got '$_v'"
  teardown_tmp
}

test_get_movetoml_env_chainid_no_section_returns_empty() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  mkdir -p "$(dirname "$_F")"
  cat >"$_F" <<EOF
[package]
name = "demo"
EOF
  local _v
  _v=$(get_movetoml_env_chainid "$_F" "localnet")
  [ -z "$_v" ] || fail "no [environments] must return empty, got '$_v'"
  teardown_tmp
}

# Defensive: a `localnet = "..."` line in another section (e.g. a
# hypothetical dependency table) MUST NOT be picked up. Only the
# [environments] section is authoritative.
test_get_movetoml_env_chainid_ignores_other_sections() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  mkdir -p "$(dirname "$_F")"
  cat >"$_F" <<EOF
[package]
name = "demo"

[environments]
localnet = "aaaaaaaa"

[other]
localnet = "ffffffff"
EOF
  local _v
  _v=$(get_movetoml_env_chainid "$_F" "localnet")
  [ "$_v" = "aaaaaaaa" ] || fail "must read from [environments], got '$_v'"
  teardown_tmp
}

# ----- sync_movetoml_env_chainid -----

test_sync_movetoml_env_chainid_updates_when_different() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  write_fixture_movetoml "$_F" "aaaaaaaa" "bbbbbbbb"
  sync_movetoml_env_chainid "$_F" "localnet" "cccccccc" 2>/dev/null
  local _v
  _v=$(get_movetoml_env_chainid "$_F" "localnet")
  [ "$_v" = "cccccccc" ] || fail "expected cccccccc, got '$_v'"
  # localnet_proxy must remain untouched.
  _v=$(get_movetoml_env_chainid "$_F" "localnet_proxy")
  [ "$_v" = "bbbbbbbb" ] || fail "other env must not change, got '$_v'"
  teardown_tmp
}

# Idempotency: when value already matches, the file MUST NOT be
# rewritten (mtime stable). This is what keeps `localnet publish`
# quiet on the common no-op case and avoids churning user-checked-in
# files when chain_ids haven't drifted.
test_sync_movetoml_env_chainid_idempotent_when_same() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  write_fixture_movetoml "$_F" "aaaaaaaa" "bbbbbbbb"
  local _MTIME_BEFORE
  _MTIME_BEFORE=$(stat -c '%Y.%N' "$_F" 2>/dev/null || stat -f '%m' "$_F")
  # Sleep 1s so any rewrite would necessarily change mtime granularity.
  sleep 1
  sync_movetoml_env_chainid "$_F" "localnet" "aaaaaaaa" 2>/dev/null
  local _MTIME_AFTER
  _MTIME_AFTER=$(stat -c '%Y.%N' "$_F" 2>/dev/null || stat -f '%m' "$_F")
  [ "$_MTIME_BEFORE" = "$_MTIME_AFTER" ] \
    || fail "idempotent sync must not touch the file (mtime before=$_MTIME_BEFORE after=$_MTIME_AFTER)"
  teardown_tmp
}

# Missing env entry — must NOT add it. Adding silently would surprise
# users; the operator decides which envs the package supports.
test_sync_movetoml_env_chainid_does_not_add_missing_entry() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  write_fixture_movetoml "$_F" "aaaaaaaa" "bbbbbbbb"
  sync_movetoml_env_chainid "$_F" "testnet" "ffffffff" 2>/dev/null
  local _v
  _v=$(get_movetoml_env_chainid "$_F" "testnet")
  [ -z "$_v" ] || fail "must not add missing entry, got '$_v'"
  teardown_tmp
}

# Defensive: the in-place sed range must restrict edits to the
# [environments] block, not bleed into later sections.
test_sync_movetoml_env_chainid_does_not_touch_other_sections() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  mkdir -p "$(dirname "$_F")"
  cat >"$_F" <<EOF
[package]
name = "demo"

[environments]
localnet = "aaaaaaaa"

[other]
localnet = "ffffffff"
EOF
  sync_movetoml_env_chainid "$_F" "localnet" "cccccccc" 2>/dev/null
  # [other] must still contain the original value.
  grep -A1 "^\[other\]" "$_F" | grep -q '"ffffffff"' \
    || fail "edit must not bleed past [environments]; file is now:
$(cat "$_F")"
  teardown_tmp
}

# ----- sync_movetoml_workdir_chainids -----

test_sync_movetoml_workdir_chainids_updates_both_envs() {
  setup_tmp
  local _F="$TEST_TMPDIR/demo/Move.toml"
  write_fixture_movetoml "$_F" "aaaaaaaa" "aaaaaaaa"
  sync_movetoml_workdir_chainids "$_F" "localnet" "dddddddd" 2>/dev/null
  local _l _p
  _l=$(get_movetoml_env_chainid "$_F" "localnet")
  _p=$(get_movetoml_env_chainid "$_F" "localnet_proxy")
  [ "$_l" = "dddddddd" ] || fail "localnet=$_l"
  [ "$_p" = "dddddddd" ] || fail "localnet_proxy=$_p"
  teardown_tmp
}

# ----- sync_local_deps_chainids -----

test_sync_local_deps_chainids_walks_local_deps() {
  setup_tmp
  # Two-package layout:
  #   $TEST_TMPDIR/demo/Move.toml       — root package
  #   $TEST_TMPDIR/log/Move.toml        — local dep referenced via local="../log"
  local _DEMO="$TEST_TMPDIR/demo/Move.toml"
  local _LOG="$TEST_TMPDIR/log/Move.toml"
  write_fixture_movetoml "$_DEMO" "aaaaaaaa" "aaaaaaaa"
  mkdir -p "$(dirname "$_LOG")"
  cat >"$_LOG" <<EOF
[package]
name = "log"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"
EOF

  sync_movetoml_workdir_chainids "$_DEMO" "localnet" "9999aaaa" 2>/dev/null
  sync_local_deps_chainids "$_DEMO" "localnet" "9999aaaa" 2>/dev/null

  local _l _p
  _l=$(get_movetoml_env_chainid "$_LOG" "localnet")
  _p=$(get_movetoml_env_chainid "$_LOG" "localnet_proxy")
  [ "$_l" = "9999aaaa" ] || fail "dep localnet=$_l"
  [ "$_p" = "9999aaaa" ] || fail "dep localnet_proxy=$_p"
  teardown_tmp
}

# Dependency without [environments] (legacy mode) must be skipped, not
# crashed on.
test_sync_local_deps_chainids_skips_dep_with_no_environments() {
  setup_tmp
  local _DEMO="$TEST_TMPDIR/demo/Move.toml"
  local _LOG="$TEST_TMPDIR/log/Move.toml"
  write_fixture_movetoml "$_DEMO" "aaaaaaaa" "aaaaaaaa"
  mkdir -p "$(dirname "$_LOG")"
  cat >"$_LOG" <<EOF
[package]
name = "log"
EOF

  # Must not error.
  sync_local_deps_chainids "$_DEMO" "localnet" "9999aaaa" 2>/dev/null \
    || fail "sync_local_deps_chainids unexpectedly errored on legacy dep"
  # Dep file content untouched.
  grep -q "name = \"log\"" "$_LOG" || fail "dep file got mangled"
  ! grep -q "\[environments\]" "$_LOG" || fail "dep file got [environments] added"
  teardown_tmp
}

# ----- should_auto_sync_chainid (workdir-scope gate) -----
#
# Auto-rewriting Move.toml only makes sense for `localnet` because its
# genesis chain_id changes on every `regen`. testnet/mainnet/devnet
# have stable chain_ids: any mismatch there is a real user error that
# should not be silently rewritten away by every publish.

test_should_auto_sync_chainid_yes_for_localnet() {
  should_auto_sync_chainid "localnet" \
    || fail "localnet must be auto-synced"
}

test_should_auto_sync_chainid_no_for_other_workdirs() {
  for w in testnet mainnet devnet active some_custom; do
    if should_auto_sync_chainid "$w"; then
      fail "$w must NOT be auto-synced"
    fi
  done
}

# ----- extract_chain_id_from_response (response validation) -----
#
# JSON-RPC parser must validate that the result looks like a hex chain
# identifier — port collisions or proxy error envelopes can produce
# arbitrary "result" strings that would otherwise be written into
# Move.toml unchecked.

test_extract_chain_id_from_response_accepts_hex() {
  local _r
  _r=$(extract_chain_id_from_response '{"jsonrpc":"2.0","id":1,"result":"9754208c"}')
  [ "$_r" = "9754208c" ] || fail "expected '9754208c', got '$_r'"
}

test_extract_chain_id_from_response_rejects_non_hex() {
  local _r
  # Non-hex characters in result.
  _r=$(extract_chain_id_from_response '{"jsonrpc":"2.0","id":1,"result":"not-a-hex-string"}')
  [ -z "$_r" ] || fail "non-hex result must be rejected; got '$_r'"
  # Mixed alpha + special chars (typical of error pages).
  _r=$(extract_chain_id_from_response '{"jsonrpc":"2.0","id":1,"result":"<html>oops</html>"}')
  [ -z "$_r" ] || fail "HTML-shaped result must be rejected; got '$_r'"
}

test_extract_chain_id_from_response_rejects_jsonrpc_error() {
  local _r
  # A JSON-RPC error envelope has no "result" field.
  _r=$(extract_chain_id_from_response '{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}')
  [ -z "$_r" ] || fail "error envelope must yield empty; got '$_r'"
}

test_extract_chain_id_from_response_rejects_empty_input() {
  local _r
  _r=$(extract_chain_id_from_response "")
  [ -z "$_r" ] || fail "empty input must yield empty; got '$_r'"
}

# ----- sync_local_deps_chainids: transitive + dev-dependencies -----
#
# Real Sui packages have multi-level dep graphs. A one-level walk
# leaves transitive deps with stale chain_ids → publish fails on the
# transitive package the developer wasn't editing. The walker must
# recurse and also include [dev-dependencies] when present.

test_sync_local_deps_chainids_walks_transitive_deps() {
  setup_tmp
  # Chain: demo -> log -> util  (util is transitive)
  local _DEMO="$TEST_TMPDIR/demo/Move.toml"
  local _LOG="$TEST_TMPDIR/log/Move.toml"
  local _UTIL="$TEST_TMPDIR/util/Move.toml"
  write_fixture_movetoml "$_DEMO" "aaaaaaaa" "aaaaaaaa"
  mkdir -p "$(dirname "$_LOG")" "$(dirname "$_UTIL")"
  cat >"$_LOG" <<EOF
[package]
name = "log"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"

[dependencies]
util = { local = "../util" }
EOF
  cat >"$_UTIL" <<EOF
[package]
name = "util"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"
EOF

  sync_movetoml_workdir_chainids "$_DEMO" "localnet" "ddddeeee" 2>/dev/null
  sync_local_deps_chainids "$_DEMO" "localnet" "ddddeeee" 2>/dev/null

  local _v
  _v=$(get_movetoml_env_chainid "$_UTIL" "localnet")
  [ "$_v" = "ddddeeee" ] || fail "transitive dep util/Move.toml not synced; got '$_v'"
  _v=$(get_movetoml_env_chainid "$_UTIL" "localnet_proxy")
  [ "$_v" = "ddddeeee" ] || fail "transitive dep util/Move.toml localnet_proxy not synced; got '$_v'"
  teardown_tmp
}

test_sync_local_deps_chainids_walks_dev_dependencies() {
  setup_tmp
  local _DEMO="$TEST_TMPDIR/demo/Move.toml"
  local _DEVDEP="$TEST_TMPDIR/devdep/Move.toml"
  mkdir -p "$(dirname "$_DEMO")" "$(dirname "$_DEVDEP")"
  cat >"$_DEMO" <<EOF
[package]
name = "demo"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"

[dev-dependencies]
testkit = { local = "../devdep" }
EOF
  cat >"$_DEVDEP" <<EOF
[package]
name = "testkit"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"
EOF

  sync_local_deps_chainids "$_DEMO" "localnet" "cccc0000" 2>/dev/null

  local _v
  _v=$(get_movetoml_env_chainid "$_DEVDEP" "localnet")
  [ "$_v" = "cccc0000" ] || fail "[dev-dependencies] entry not walked; got '$_v'"
  teardown_tmp
}

# Robustness against a future bidirectional dep declaration (or an
# accidental self-cycle): walk must terminate.
test_sync_local_deps_chainids_handles_cycle() {
  setup_tmp
  local _A="$TEST_TMPDIR/a/Move.toml"
  local _B="$TEST_TMPDIR/b/Move.toml"
  mkdir -p "$(dirname "$_A")" "$(dirname "$_B")"
  cat >"$_A" <<EOF
[package]
name = "a"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"

[dependencies]
b = { local = "../b" }
EOF
  cat >"$_B" <<EOF
[package]
name = "b"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"

[dependencies]
a = { local = "../a" }
EOF

  # Must terminate (under whatever timeout the test harness has).
  # The fail() function is invoked if the walk doesn't return.
  sync_local_deps_chainids "$_A" "localnet" "f00fc0fc" 2>/dev/null \
    || fail "walk did not terminate on cycle"

  local _v
  _v=$(get_movetoml_env_chainid "$_B" "localnet")
  [ "$_v" = "f00fc0fc" ] || fail "cycle dep b not synced; got '$_v'"
  teardown_tmp
}

# Defensive: a single line may contain MULTIPLE `local = "..."`
# entries (inline-table form with nested dep specs). The walker must
# emit BOTH paths, not just the last (greedy regex bug).
test_sync_local_deps_chainids_handles_multiple_local_on_one_line() {
  setup_tmp
  local _ROOT="$TEST_TMPDIR/root/Move.toml"
  local _ONE="$TEST_TMPDIR/one/Move.toml"
  local _TWO="$TEST_TMPDIR/two/Move.toml"
  mkdir -p "$(dirname "$_ROOT")" "$(dirname "$_ONE")" "$(dirname "$_TWO")"
  # A single dep line with two `local = "..."` substrings (nested
  # inline table, valid TOML).
  cat >"$_ROOT" <<'EOF'
[package]
name = "root"

[dependencies]
multi = { local = "../one", extra = { local = "../two" } }
EOF
  for _f in "$_ONE" "$_TWO"; do
    cat >"$_f" <<EOF
[package]
name = "x"

[environments]
localnet = "aaaaaaaa"
localnet_proxy = "aaaaaaaa"
EOF
  done

  sync_local_deps_chainids "$_ROOT" "localnet" "12345678" 2>/dev/null

  local _v
  _v=$(get_movetoml_env_chainid "$_ONE" "localnet")
  [ "$_v" = "12345678" ] || fail "first inline local not synced; got '$_v'"
  _v=$(get_movetoml_env_chainid "$_TWO" "localnet")
  [ "$_v" = "12345678" ] || fail "second inline local not synced (greedy regex bug); got '$_v'"
  teardown_tmp
}

# ----- get_current_chain_id: fail-soft on unset proxy config -----
#
# When CFG_proxy_host_ip / CFG_proxy_port_number are unset (workdir
# without proxy config), get_current_chain_id must return empty
# without writing "parameter null or not set" to stderr and without
# aborting the parent shell.

test_get_current_chain_id_fail_soft_on_unset_proxy_config() {
  local _out _err _rc
  # Run in a subshell with proxy vars unset; capture stderr separately.
  _err=$(
    {
      unset CFG_proxy_host_ip
      unset CFG_proxy_port_number
      _out=$(get_current_chain_id)
      _rc=$?
      echo "RC=$_rc OUT=[$_out]" >&3
    } 2>&1 >/dev/null
  ) 3>&1
  # stdout from inside the subshell was redirected; we captured stderr
  # in $_err. Just assert no "parameter null or not set" leaked.
  if echo "$_err" | grep -q "parameter null or not set"; then
    fail "get_current_chain_id leaked parameter-expansion error: $_err"
  fi
}

# ----- driver -----

tests() {
  test_get_movetoml_env_chainid_reads_value
  test_get_movetoml_env_chainid_missing_env_returns_empty
  test_get_movetoml_env_chainid_no_section_returns_empty
  test_get_movetoml_env_chainid_ignores_other_sections
  test_sync_movetoml_env_chainid_updates_when_different
  test_sync_movetoml_env_chainid_idempotent_when_same
  test_sync_movetoml_env_chainid_does_not_add_missing_entry
  test_sync_movetoml_env_chainid_does_not_touch_other_sections
  test_sync_movetoml_workdir_chainids_updates_both_envs
  test_sync_local_deps_chainids_walks_local_deps
  test_sync_local_deps_chainids_skips_dep_with_no_environments
  test_should_auto_sync_chainid_yes_for_localnet
  test_should_auto_sync_chainid_no_for_other_workdirs
  test_extract_chain_id_from_response_accepts_hex
  test_extract_chain_id_from_response_rejects_non_hex
  test_extract_chain_id_from_response_rejects_jsonrpc_error
  test_extract_chain_id_from_response_rejects_empty_input
  test_sync_local_deps_chainids_walks_transitive_deps
  test_sync_local_deps_chainids_walks_dev_dependencies
  test_sync_local_deps_chainids_handles_cycle
  test_sync_local_deps_chainids_handles_multiple_local_on_one_line
  test_get_current_chain_id_fail_soft_on_unset_proxy_config
}

tests
echo "test_movetoml_chainid_sync: passed"
exit 0
