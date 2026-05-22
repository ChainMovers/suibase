#!/bin/bash

# Common code for TypeScript helper test scripts.
#
# The first parameter is the directory where the npm-based tests will be run.

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
NPM_DIR="$1"
shift

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

if [ "$FAST_OPTION" = "true" ]; then
  echo "Skipping $NPM_DIR (fast option)"
  return 2
fi

# Helper tests must run against localnet (demo package is published there).
if [ "$WORKDIR" != "localnet" ]; then
  echo "Skipping $NPM_DIR (not localnet)"
  return 2
fi

# Skip if Node.js is not available. The TypeScript helper is opt-in:
# suibase end-users do not need Node.js installed.
if ! command -v node >/dev/null 2>&1; then
  echo "Skipping $NPM_DIR (node not installed)"
  return 2
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "Skipping $NPM_DIR (npm not installed)"
  return 2
fi

# Require Node >= 20.
_node_major=$(node -e 'process.stdout.write(String(process.versions.node.split(".")[0]))' 2>/dev/null)
if [ -z "$_node_major" ] || [ "$_node_major" -lt 20 ]; then
  echo "Skipping $NPM_DIR (node v$_node_major < 20)"
  return 2
fi

localnet start
localnet set-active

# Helper integration tests require the 'demo' package to be published.
if [ ! -d "$HOME/suibase/workdirs/localnet/published-data/demo" ]; then
  cd "$HOME/suibase/rust/demo-app" || fail "'cd $HOME/suibase/rust/demo-app' failed"
  localnet publish || fail "'localnet publish' failed for demo"
fi
# Confirm demo really got published (localnet publish can return 0 even on
# protocol errors). The integration test needs the 'most-recent' symlink.
if [ ! -L "$HOME/suibase/workdirs/localnet/published-data/demo/most-recent" ]; then
  fail "'localnet publish' did not create demo/most-recent symlink"
fi

do_tests() {
  update_HOST_vars

  # Run npm commands in a subshell, but capture its exit code so we
  # do not silently swallow failures. `fail` inside a (...) only exits
  # the subshell.
  (
    set -e
    cd "$NPM_DIR"
    npm install --no-audit --no-fund
    npm run typecheck
    npm test
  ) || fail "'npm test' failed in $NPM_DIR"

  assert_workdir_ok "$WORKDIR"
}

do_tests
