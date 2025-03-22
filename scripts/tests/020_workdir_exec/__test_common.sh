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

# When testing for release, just validate with localnet and testnet.
if [ "$RELEASE_TESTS_OPTION" = "true" ]; then
  if [ "$WORKDIR" != "localnet" ] && [ "$WORKDIR" != "testnet" ]; then
    return 2
  fi
fi

# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

test_suibase_yaml() {
  # Test that suibase.yaml is present and has the expected content.
  assert_file_exists "$WORKDIRS/$WORKDIR/suibase.yaml"
  # clear_suibase_yaml
  clear_sui_keystore
  add_to_suibase_yaml "add_private_keys:"
  add_to_suibase_yaml "  - 0x0cdb9491ab9697379802b188cd3566920cbb095dccca3fd91765bb45b461c30f"
  ($WORKDIR update) || fail "$WORKDIR update failed"
  assert_file_contains "$WORKDIRS/$WORKDIR/config/sui.keystore" "AAzblJGrlpc3mAKxiM01ZpIMuwldzMo/2Rdlu0W0YcMP"
}

test_autocoins_commands_gating() {
  # Verify that only testnet return success when $WORKDIR = "testnet":
  #  $WORKDIR autocoins enable / disable / purge-data / set / status
  #
  # For all other $WORKDIR, must return failure status code.
  if [ "$WORKDIR" = "testnet" ]; then
    $WORKDIR autocoins || fail "$WORKDIR autocoins failed"
    $WORKDIR autocoins enable || fail "$WORKDIR autocoins enable failed"
    $WORKDIR autocoins disable || fail "$WORKDIR autocoins disable failed"
    $WORKDIR autocoins purge-data || fail "$WORKDIR autocoins purge-data failed"
    $WORKDIR autocoins set "0xec2d64579ec698231bec97dabc1c18b0515ac4b1af17e99feab3242248596ae5" || fail "$WORKDIR autocoins set <address> failed"
    $WORKDIR autocoins status || fail "$WORKDIR autocoins status failed"
  else
    $WORKDIR autocoins && fail "$WORKDIR autocoins should have failed"
    $WORKDIR autocoins enable && fail "$WORKDIR autocoins enable should have failed"
    $WORKDIR autocoins disable && fail "$WORKDIR autocoins disable should have failed"
    $WORKDIR autocoins purge-data && fail "$WORKDIR autocoins purge-data should have failed"
    $WORKDIR autocoins set "0xec2d64579ec698231bec97dabc1c18b0515ac4b1af17e99feab3242248596ae5" && fail "$WORKDIR autocoins set <address> should have failed"
    $WORKDIR autocoins status && fail "$WORKDIR autocoins status should have failed"
  fi
}

tests() {
  # Make sure $WORKDIR is stop.
  # This will allow to apply config changes (if any) on next start.
  $WORKDIR stop || fail "$WORKDIR stop failed"

  if [ -d "$WORKDIRS/$WORKDIR" ] && [ ! -f "$WORKDIRS/$WORKDIR/suibase.yaml" ]; then
    # This was broken at some point in the past, so check for a fix
    # in repair_workdir_as_needed()
    echo "Workdir $WORKDIR already exists, but suibase.yaml missing."
    exit 1
  fi

  # Just run most commands and look for a failure.
  ($WORKDIR start) || fail "$WORKDIR start failed"
  assert_workdir_ok "$WORKDIR"
  ($WORKDIR set-active) || fail "$WORKDIR set-active failed"
  assert_build_ok "$WORKDIR"
  ($WORKDIR status) || fail "$WORKDIR status failed"
  $WORKDIR stop || fail "$WORKDIR stop failed"

  # Verify still healthy.
  assert_workdir_ok "$WORKDIR"

  test_suibase_yaml

  # TODO re-enable after fixing the test code itself!
  # test_autocoins_commands_gating

  # Clean-up to make disk space... except for localnet.
  # if [ "$WORKDIR" != "localnet" ]; then
  #  $WORKDIR delete || fail "$WORKDIR delete failed"
  # fi
}

tests
