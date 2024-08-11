#!/bin/bash

# Tests for ~/suibase/install
#
# At the end of this script, a valid localnet installation should be in place.
#
SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# Installation.
(~/suibase/install >"$OUT" 2>&1) || fail "install exit status=[$?]"

# As needed, create scripts/templates/common/suibase.yaml
init_common_template

# Source globals
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

tests() {
  # shellcheck source=SCRIPTDIR/../../../../suibase/install
  test_no_workdirs
  if ! $SCRIPTS_TESTS_OPTION; then
    return;
  fi
}

delete_workdirs() {
  echo "Deleting workdirs"
  ~/suibase/scripts/dev/stop-daemon
  rm -rf ~/suibase/workdirs >/dev/null 2>&1
  # Display the content of workdirs (recursively) if still exists.
  if [ -d "$HOME/suibase/workdirs" ]; then
    echo "Workdirs deletion failed. Files remaining:"
    ls -lR ~/suibase/workdirs
  fi
}

test_no_workdirs() {
  rm -rf ~/suibase/workdirs >/dev/null 2>&1
  # Make sure not in a directory that was deleted.
  cd "$HOME/suibase" || fail "cd $HOME/suibase failed"

  if $SCRIPTS_TESTS_OPTION; then
    delete_workdirs
    (localnet update >"$OUT" 2>&1) || fail "update"
    assert_workdir_ok "localnet"
    assert_build_ok "localnet"

    # Will cleanly stop the processes (if any).
    echo "localnet delete"
    (localnet delete >"$OUT" 2>&1) || fail "delete"

    delete_workdirs
    echo "localnet start"
    (localnet start >"$OUT" 2>&1) || fail "start"
    assert_workdir_ok "localnet"
    assert_build_ok "localnet"

    echo "localnet stop"
    (localnet stop >"$OUT" 2>&1) || fail "stop"
    assert_workdir_ok "localnet"
    assert_build_ok "localnet"
  fi

  delete_workdirs
  echo "localnet create"
  (localnet create >"$OUT" 2>&1) || fail "create"
  assert_workdir_ok "localnet"
}
export -f test_no_workdirs

tests
