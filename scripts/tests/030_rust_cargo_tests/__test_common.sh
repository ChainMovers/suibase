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

localnet start
localnet set-active

if [ "$FAST_OPTION" = "true" ]; then
  if [[ "$CARGO_DIR" == *"demo-app"* ]]; then
    return 2
  fi
fi

# helper and demo-app integration tests requires the package 'demo' to be published.
if [[ "$CARGO_DIR" == *"demo-app"* ]] || [[ "$CARGO_DIR" == *"helper"* ]]; then
  if [ ! -d "$HOME/suibase/workdirs/localnet/published-data/demo" ]; then    
    cd "$HOME/suibase/rust/demo-app" || fail "'cd $HOME/suibase/rust/demo-app' failed"
    localnet publish
    # TODO verify that the publication was successful.
  fi
fi

do_tests() {
  # Do 'cargo clippy', but only on Linux (somehow not always installed on Apple/Darwin).
  # TODO detect if "cargo clippy" installed instead.
  update_HOST_vars
  if [ "$HOST_PLATFORM" = "Linux" ]; then
    (
      cd "$CARGO_DIR" || fail "'cd $CARGO_DIR' failed for 'cargo clippy'"
      cargo clippy -- -D warnings || fail "'$CARGO_DIR/cargo clippy' failed"
    )
  fi

  # Do 'cargo test'
  (
    cd "$CARGO_DIR" || fail "'cd $CARGO_DIR' failed for 'cargo test'"
    cargo test || fail "'$CARGO_DIR/cargo test' failed"
  )

  # Verify still healthy.
  assert_workdir_ok "$WORKDIR"

  # Do 'cargo clean'
  (
    cd "$CARGO_DIR" || fail "'cd $CARGO_DIR' failed for 'cargo clean'"
    cargo clean || fail "'$CARGO_DIR/cargo clean' failed"
  )

}

do_tests
