#!/bin/bash

# Tests for ~/suibase/install
#
# At the end of this script, a valid installation should be in place.
#
SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# Installation.
(~/suibase/install >&"$OUT") || fail "install exit status=[$?]"

# As needed, create scripts/templates/common/suibase.yaml
init_common_template

# Source globals
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

tests() {
  # shellcheck source=SCRIPTDIR/../../../../suibase/install
  test_no_workdirs
}

test_no_workdirs() {
  rm -rf ~/suibase/workdirs
  # Make sure not in a directory that was deleted.
  cd "$HOME/suibase" || fail "cd $HOME/suibase failed"

  echo "localnet create"
  (localnet create >&"$OUT") || fail "create"
  assert_workdir_ok "localnet"

  #rm -rf ~/suibase/workdirs
  #echo "localnet update"
  #(localnet update >& "$OUT") || fail "update"
  #assert_workdir_ok "localnet"
  #assert_build_ok "localnet"
}
export -f test_no_workdirs

tests
