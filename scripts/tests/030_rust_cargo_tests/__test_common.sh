#!/bin/bash

# Common code for other test script in this directory.

# The first parameter is the directory where the "cargo test" will be done.

# Tests for workdir commands (e.g. localnet, testnet, etc.)
SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/common/__globals.sh
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
CARGO_DIR="$1"
shift

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

do_tests() {
  # Do 'cargo clippy'
  (
    cd "$CARGO_DIR" || fail "'cd $CARGO_DIR' failed for 'cargo clippy'"
    cargo clippy -- -D warnings || fail "'$CARGO_DIR/cargo clippy' failed"
  )

  # Do 'cargo test'
  (
    cd "$CARGO_DIR" || fail "'cd $CARGO_DIR' failed for 'cargo test'"
    cargo test || fail "'$CARGO_DIR/cargo test' failed"
  )

  # Verify still healthy.
  assert_workdir_ok "$WORKDIR"
}

do_tests
