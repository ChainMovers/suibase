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
}

tests
echo "test_movetoml_chainid_sync: passed"
exit 0
