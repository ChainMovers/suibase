#!/bin/bash

# Unit tests for common/__globals.sh

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../../common/__scripts-tests.sh
source "$SUIBASE_DIR/scripts/common/__scripts-tests.sh"

# shellcheck source=SCRIPTDIR/common/__globals.sh
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="active"
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

tests() {
  test_setup "$@"
  test_workdir "localnet"
  if [ "$MAIN_BRANCH_OPTION" = "true" ]; then
    return
  fi

  test_workdir "mainnet"
  if [ "$FAST_OPTION" = "true" ]; then
    return
  fi

  test_workdir "devnet"
  test_workdir "testnet"
}

test_workdir() {
  local _WORKDIR="$1"

  # Just run most commands and look for a failure.
  $_WORKDIR start || fail "$_WORKDIR start failed"
  assert_workdir_ok "$_WORKDIR"
  assert_build_ok "$_WORKDIR"
  $_WORKDIR status || fail "$_WORKDIR status failed"
  $_WORKDIR stop || fail "$_WORKDIR stop failed"

}
export -f test_workdir

tests "$@"
