#!/bin/bash

# Tests for ~/suibase/install
#
# At the end of this script, a valid installation should be in place.
#
SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../../common/__scripts-tests.sh
source "$SUIBASE_DIR/scripts/common/__scripts-tests.sh"

test_no_workdirs() {
    rm -rf ~/suibase/workdirs
    echo "localnet create"
    (localnet create >&"$OUT") || fail "create"
    assert_workdir_ok "localnet"

    #rm -rf ~/suibase/workdirs
    #echo "localnet update"
    #(localnet update >& "$OUT") || fail "update"
    #assert_workdir_ok "localnet"
    #assert_build_ok "localnet"
}

tests() {
    test_setup "$@"
    # shellcheck source=SCRIPTDIR/../../../../suibase/install
    (~/suibase/install >&"$OUT") || fail "install exit status=[$?]"
    # As needed, create scripts/templates/common/suibase.yaml
    init_common_template
    test_no_workdirs
}

tests "$@"
