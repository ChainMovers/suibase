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
    test_static_globals_var
}

test_static_globals_var() {
    # These are all the variables that should always be set
    # upon sourcing __globals.sh
    local _STATIC_GLOBALS_VAR=(
        "SUIBASE_DIR"
        "USER_CWD"
        "SUIBASE_VERSION"
        "MIN_SUI_VERSION"
        "MIN_RUST_VERSION"
        "SCRIPT_PATH"
        "SCRIPT_NAME"
        "WORKDIR"
        "SUIBASE_DIR"
        "WORKDIRS"
        "LOCAL_BIN_DIR"
        "SCRIPTS_DIR"
        "SUI_REPO_DIR"
        "CONFIG_DATA_DIR"
        "PUBLISHED_DATA_DIR"
        "FAUCET_DIR"
        "SUI_BIN_DIR"
        "SUI_BIN_ENV"
        "SUIBASE_BIN_DIR"
        "SUIBASE_LOGS_DIR"
        "SUIBASE_TMP_DIR"
        "SUIBASE_DAEMON_NAME"
        "SUIBASE_DAEMON_BUILD_DIR"
        "SUIBASE_DAEMON_BIN"
        "WORKDIR_NAME"
        "SUI_SCRIPT"
        "NETWORK_CONFIG"
        "CLIENT_CONFIG"
        "SUI_REPO_DIR_DEFAULT"
        "CONFIG_DATA_DIR_DEFAULT"
        "DEFAULT_GENESIS_DATA_DIR"
        "GENERATED_GENESIS_DATA_DIR"
        "SUI_EXEC"
        "WORKDIR_EXEC"
        "SUI_CLIENT_LOG_DIR"
        "SUI_BASE_NET_MOCK"
        "SUI_BASE_NET_MOCK_VER"
        "SUI_BASE_NET_MOCK_PID"
        "NOLOG_KEYTOOL_BIN"
    )

    # All vars in _STATIC_GLOBALS_VAR should be set to something.
    for _VAR in "${_STATIC_GLOBALS_VAR[@]}"; do
        if [ -z "${!_VAR}" ]; then
            fail "Variable $_VAR is empty"
        fi
    done

    return 0 # Success
}
export -f test_static_globals_var

tests
