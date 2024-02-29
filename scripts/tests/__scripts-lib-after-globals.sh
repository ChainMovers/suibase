#!/bin/bash

# Library script to assist other test script.
#
# This is not intended to be used directly by the user.
#

# common/__globals must be sourced before this file.
if [ -z "$SUIBASE_VERSION" ]; then
  fail "Incorrect sourcing order for __scripts-test-after-globals.sh"
fi

# List of workdir routinely tested.
export WORKDIRS_NAME=(
  "localnet"
  "devnet"
  "testnet"
  "mainnet"
)

assert_workdir_ok() {
  # shellcheck disable=SC2153
  local _DIR="$WORKDIRS/$1"

  # Verify minimal integrity of workdirs/_DIR.
  [ -d "$WORKDIRS" ] || fail "workdirs missing"
  [ -L "$WORKDIRS/active" ] || fail "workdirs/active missing"
  [ -d "$_DIR" ] || fail "workdirs/localnet missing"

  [ -f "$_DIR/sui-exec" ] || fail "workdirs/sui-exec missing"
  [ -x "$_DIR/sui-exec" ] || fail "workdirs/sui-exec not exec"

  [ -f "$_DIR/workdir-exec" ] || fail "workdirs/workdir-exec missing"
  [ -x "$_DIR/workdir-exec" ] || fail "workdirs/workdir-exec not exec"

  [ -f "$_DIR/suibase.yaml" ] || fail "workdirs/suibase.yaml missing"

  # Change to a directory known to exists (prevents failures of GETCWD)
  # cd "$_DIR" || fail "cd _DIR failed"

  # First word out of "workdir-exec" should be the workdir name

  local _HELP
  _HELP=$("$_DIR"/workdir-exec)
  _RESULT="$?"
  if [ ! "$_RESULT" -eq 0 ]; then
    fail "workdir-exec usage should not be an error"
  fi
  _FIRST_WORD=$(echo "$_HELP" | head -n1 | awk '{print $1;}')
  # Note: Must use contain because of the ANSI color escape code.
  [[ "$_FIRST_WORD" == *"$1"* ]] || fail "usage first word [$_FIRST_WORD] not [$1]"

  # Usage should have the suibase version, so sanity verify for "suibase"
  [[ "$_HELP" == *"suibase"* ]] || fail "usage does not mention suibase [$_HELP]"
}

assert_build_ok() {
  local _WORKDIR="$1"
  local _DIR="$WORKDIRS/$_WORKDIR"
  local _SUI_BIN="$_DIR/sui-repo/target/debug/sui"

  if [ ! -d "$_DIR/sui-repo" ]; then
    fail "sui-repo missing"
  fi

  # Verify that the Sui binary execution is OK.
  local _VERSION _FIRST_WORD
  _VERSION=$($_SUI_BIN --version)
  _FIRST_WORD=$(echo "$_VERSION" | head -n1 | awk '{print $1;}')
  [ "$_FIRST_WORD" = "sui" ] || fail "sui --version did not work [$_VERSION]"
  if [ "${CFG_default_repo_branch:?}" = "main" ]; then
    # "Cutting edge" branch is not precompiled by Mysten Labs.
    local _PRECOMP_STATE
    _PRECOMP_STATE=$(get_key_value "$_WORKDIR" "precompiled")
    if [ "$_PRECOMP_STATE" != "NULL" ]; then
      fail ".state/precompiled should not be set for main branch [$_PRECOMP_STATE]"
    fi
  fi
}

add_to_suibase_yaml() {
  # Append $1 string to suibase.yaml
  echo "$1" >>"$WORKDIRS/$WORKDIR/suibase.yaml"
}

clear_suibase_yaml() {
  # Clear suibase.yaml, keep the file
  echo "# Cleared by automated tests" >"$WORKDIRS/$WORKDIR/suibase.yaml"
}

clear_sui_keystore() {
  # Clear sui.keystore, keep the file.
  echo "[]" >"$WORKDIRS/$WORKDIR/config/sui.keystore"
}
