#!/bin/bash

# Mostly lightweight unit tests for common/__globals.sh utilities that do
# not depend on building the sui binaries.
SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# Source globals
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

tests() {
  test_file_newer_than_phase_1
  test_static_globals_var
  test_color
  test_string_utils
  test_has_param
  cd "$HOME" || fail "cd $HOME"
  rm -rf "${WORKDIRS:?}"
  localnet create || fail "localnet create"

  # Some tests are in two phases, with *at least* one second
  # one second apart from the first phase.
  sleep 1
  test_file_newer_than_phase_2
}

test_color() {
  # Just call every color function to make sure they do not exit.
  echo
  echo_black " black "
  echo_red " red "
  echo_green " green "
  echo_yellow " yellow "
  echo_blue " blue "
  echo_magenta " magenta "
  echo_cyan " cyan "
  echo_white " white "
  echo_low_green " low green "
  echo_low_yellow " low yellow "
  echo
}

test_version_utils() {
  if ! version_less_than "1.0.0" "2.0.0"; then
    fail "1.0.0 should be less than 2.0.0"
  fi

  if version_less_than "2.0.0" "1.0.0"; then
    fail "2.0.0 should not be less than 1.0.0"
  fi

  if version_less_than "1.0.0" "1.0.0"; then
    fail "1.0.0 should not be less than 1.0.0"
  fi

  if version_less_than "0.10.0" "0.9.0"; then
    fail "0.10.0 should not be less than 0.9.0"
  fi

  if version_less_than "0.0.10" "0.0.1"; then
    fail "0.0.10 should not be less than 0.0.1"
  fi

  if ! version_less_than "0.9.9" "1.0.0"; then
    fail "0.9.9 should be less than 1.0.0"
  fi

  if ! version_less_than "0.0.9" "0.1.0"; then
    fail "0.0.9 should be less than 0.1.0"
  fi

  if ! version_less_than "0.0.10" "0.1.0"; then
    fail "0.0.10 should be less than 0.1.0"
  fi

  if ! version_greater_equal "2.0.0" "1.0.0"; then
    fail "2.0.0 should be greater than or equal to 1.0.0"
  fi

  if ! version_greater_equal "1.0.0" "1.0.0"; then
    fail "1.0.0 should be greater than or equal to 1.0.0"
  fi

  if version_greater_equal "1.0.0" "2.0.0"; then
    fail "1.0.0 should not be greater than or equal to 2.0.0"
  fi

  if ! version_greater_equal "0.0.1" "0.0.1"; then
    fail "equality 0.0.1"
  fi

  if ! version_greater_equal "0.1.0" "0.1.0"; then
    fail "equality 0.1.0"
  fi

  if ! version_greater_equal "1.0.0" "1.0.0"; then
    fail "equality 1.0.0"
  fi

  if ! version_greater_equal "0.100.0" "0.90.0"; then
    fail "0.100.0 should not be less than 0.90.0"
  fi

  if ! version_greater_equal "0.0.100" "0.0.10"; then
    fail "0.0.100 should not be less than 0.0.10"
  fi

  if version_greater_equal "0.99.99" "1.0.0"; then
    fail "0.99.99 should be less than 1.0.0"
  fi

  if version_greater_equal "0.0.99" "0.10.0"; then
    fail "0.0.99 should be less than 0.10.0"
  fi
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
    "DEBUG_PARAM"
  )

  # All vars in _STATIC_GLOBALS_VAR should be set to something.
  for _VAR in "${_STATIC_GLOBALS_VAR[@]}"; do
    if [ -z "${!_VAR}" ]; then
      fail "Variable $_VAR is empty"
    fi
  done

  return 0 # Success
}

test_string_utils() {
  if ! beginswith "hello" "hello world"; then
    fail "beginswith failed to match 'hello' in 'hello world'"
  fi

  if beginswith "world" "hello world"; then
    fail "beginswith matched 'world' in 'hello world'"
  fi

  if ! beginswith "hello" "hello"; then
    fail "beginswith failed to match 'hello' in 'hello'"
  fi

  if ! beginswith "a" "ab"; then
    fail "beginswith failed to match 'a' in 'ab'"
  fi

  if beginswith "b" "ab"; then
    fail "beginswith matched 'b' in 'ab'"
  fi

  if ! beginswith "a" "a"; then
    fail "beginswith failed to match 'a' in 'a'"
  fi

  # Empty string does not really make sense, but just
  # test here that the behavior does not change.
  if ! beginswith "" "hello world"; then
    fail "beginswith failed to match empty string in 'hello world'"
  fi

  if ! beginswith "" ""; then
    fail "beginswith failed to match empty string with empty string"
  fi

  if beginswith "a" ""; then
    fail "beginswith matches 'a' with empty string"
  fi
}

export FILE1_TMP
export FILE2_TMP
test_file_newer_than_phase_1() {
  FILE1_TMP=$(mktemp)
}

test_file_newer_than_phase_2() {
  # FILE1_TMP and FILE2_TMP are two temporary files with
  # different modification times.
  FILE2_TMP=$(mktemp)
  sleep 1
  touch "$FILE1_TMP"

  if ! file_newer_than "$FILE1_TMP" "$FILE2_TMP"; then
    fail "$FILE1_TMP should be newer than $FILE2_TMP"
  fi

  if file_newer_than "$FILE2_TMP" "$FILE1_TMP"; then
    fail "$FILE2_TMP should not be newer than $FILE1_TMP"
  fi

  if ! file_newer_than "$FILE1_TMP" "/nonexistent/file"; then
    fail "$FILE1_TMP should be newer than /nonexistent/file"
  fi

  # This does not make sense, but test for maintaining the behavior.
  if ! file_newer_than "/nonexistent/file" "$FILE1_TMP"; then
    fail "/nonexistent/file should be newer than $FILE1_TMP"
  fi

  # Clean up the temporary files.
  rm "$FILE1_TMP" "$FILE2_TMP"
}

test_has_param() {
  if ! has_param "" "--a" "--a"; then
    fail "has_param failed to match --a"
  fi

  if ! has_param "" "--a" "--b" "--a"; then
    fail "has_param failed to match --a"
  fi

  if ! has_param "" "--a" "--b" "--a" "--c sdf"; then
    fail "has_param failed to match --a"
  fi

  if ! has_param "" "--c" "--b" "--a" "--c"; then
    fail "has_param failed to match --c"
  fi

  if ! has_param "" "--c" "--b" "--a" "--c" "c_parameter"; then
    fail "has_param failed to match --c when having a parameter"
  fi

  if ! has_param "-c" "--c" "--b" "--a" "-c" "c_parameter"; then
    fail "has_param failed to match -c short param"
  fi

  if has_param "-c" "--c" "--b" "--a" "-ca" "--ca" "--ac" "c"; then
    fail "has_param unexpected matching of -c or --c"
  fi
}

tests
