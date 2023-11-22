#!/bin/bash

# Common code for other test script in this directory.

# Tests for workdir commands (e.g. localnet, testnet, etc.)
SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/common/__globals.sh
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="$1"
shift

# When CI_WORKDIR is set, only that workdir will be done. Skip all others.
if [ -n "$CI_WORKDIR" ]; then
  if [ "$WORKDIR" != "$CI_WORKDIR" ]; then
    return 2
  fi
fi

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# Modification to suibase.yaml must be done before loading __globals.sh
if [ "$MAIN_BRANCH_OPTION" = "true" ]; then
  if [ "$WORKDIR" != "localnet" ]; then
    return 2
  fi
  # Change localnet branch to main using suibase.yaml.
  localnet create || fail "localnet create" # Create if does not already exists.
  echo 'default_repo_branch: "main"' >>"$HOME/suibase/workdirs/localnet/suibase.yaml"
fi

if [ "$FAST_OPTION" = "true" ]; then
  if [ "$WORKDIR" != "localnet" ] && [ "$WORKDIR" != "mainnet" ]; then
    return 2
  fi
fi

# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

tests() {
  # Just run most commands and look for a failure.
  ($WORKDIR start) || fail "$WORKDIR start failed"
  assert_workdir_ok "$WORKDIR"
  ($WORKDIR set-active) || fail "$WORKDIR set-active failed"
  assert_build_ok "$WORKDIR"
  ($WORKDIR status) || fail "$WORKDIR status failed"
  $WORKDIR stop || fail "$WORKDIR stop failed"

  # Verify still healthy.
  assert_workdir_ok "$WORKDIR"

  # Clean-up to make disk space... except for localnet.
  # if [ "$WORKDIR" != "localnet" ]; then
  #  $WORKDIR delete || fail "$WORKDIR delete failed"
  # fi
}

tests
