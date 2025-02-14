#!/bin/bash

# Library script to assist other test script.
#
# This is not intended to be used directly by the user.
#

# This must be sourced before common/__globals.h
if [ -n "$SUIBASE_VERSION" ]; then
  fail "Incorrect sourcing order for __scripts-test-before-globals.sh"
fi

export OUT="$HOME/suibase/scripts/tests/result.txt"

# Initialized by test_setup()
export FAST_OPTION=false
export MAIN_BRANCH_OPTION=false

export SCRIPTS_TESTS_OPTION=false
export SUIBASE_DAEMON_TESTS_OPTION=false
export RUST_TESTS_OPTION=false
export RELEASE_TESTS_OPTION=false
export MAIN_MERGE_CHECK_OPTION=false

export DEV_PUSH_CHECK_OPTION=false


export TEST_INIT_CALLED=false
export USE_GITHUB_TOKEN=""

fail() {
  echo Failed ["$1"]
  # Print stacktrace
  local i=1 line file func
  while read -r line func file < <(caller $i); do
    echo >&2 "[$i] $file:$line $func(): $(sed -n ${line}p $file)"
    ((i++))
  done

  if [ -f "$OUT" ]; then
    echo "Last stdout/stderr written to disk (may not relate to error):"
    cat "$OUT"
  fi

  exit 1
}
export -f fail

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

test_init() {
  if [ "$TEST_INIT_CALLED" = "true" ]; then
    return
  fi
  TEST_INIT_CALLED=true
  rm -rf "$OUT" >/dev/null 2>&1
  # This script should not be called from under workdirs since it will get deleted.
  local _USER_CWD
  _USER_CWD=$(pwd -P)
  if [[ "$_USER_CWD" == *"suibase/workdirs"* ]]; then
    fail "Should not call this test from a directory that will get deleted [suibase/workdirs]"
  fi
}
export -f test_init

init_common_template() {
  # As needed, create a common suibase.yaml template file.
  if [ -n "$USE_GITHUB_TOKEN" ]; then
    # Do the following only if github_token is not already in the file.
    if ! grep -q "github_token:" "$HOME/suibase/scripts/templates/common/suibase.yaml" 2>/dev/null; then
      echo "Creating templates/common/suibase.yaml"
      mkdir -p "$HOME/suibase/scripts/templates/common"
      echo "github_token: $USE_GITHUB_TOKEN" >>"$HOME/suibase/scripts/templates/common/suibase.yaml"
    fi
  fi
}
export -f init_common_template

assert_file_exists() {
  if [ ! -f "$1" ]; then
    fail "File does not exist [$1]"
  fi
}

assert_file_contains() {
  if ! grep -q "$2" "$1"; then
    fail "File '$1' does not contain '$2'"
  fi
}

init_tests_set_vars() {
  # First param should be either true/false
  SCRIPTS_TESTS_OPTION=$1
  SUIBASE_DAEMON_TESTS_OPTION=$1
  RUST_TESTS_OPTION=$1
  RELEASE_TESTS_OPTION=$1
}

test_setup_on_sourcing() {
  # Parse command-line
  FAST_OPTION=false
  MAIN_BRANCH_OPTION=false
  MAIN_MERGE_CHECK_OPTION=false
  DEV_PUSH_CHECK_OPTION=false

  init_tests_set_vars false
  local _SKIP_INIT=false
  local _ONE_TEST_SET=false
  while [[ "$#" -gt 0 ]]; do
    case $1 in
    --fast) FAST_OPTION=true ;;
    --main_branch) MAIN_BRANCH_OPTION=true ;;

    --scripts-tests) SCRIPTS_TESTS_OPTION=true; _ONE_TEST_SET=true ;;
    --suibase-daemon-tests) SUIBASE_DAEMON_TESTS_OPTION=true; _ONE_TEST_SET=true ;;
    --rust-tests) RUST_TESTS_OPTION=true; _ONE_TEST_SET=true ;;
    --release-tests) RELEASE_TESTS_OPTION=true; _ONE_TEST_SET=true ;;

    --main-merge-check) MAIN_MERGE_CHECK_OPTION=true ;;
    --dev-push-check) DEV_PUSH_CHECK_OPTION=true ;;
    --github_token)
      echo "Using GITHUB_TOKEN from command line"
      USE_GITHUB_TOKEN="$2"
      shift
      ;;
    --skip_init) _SKIP_INIT=true ;;
    *)
      fail "Unknown parameter passed: $1"
      ;;
    esac
    shift
  done

  # When no test sets is selected, then assume all tests are to be run.
  if ! $_ONE_TEST_SET; then
    # Enable them all.
    init_tests_set_vars true
  fi

  if [ -z "$USE_GITHUB_TOKEN" ] && [ -n "$GITHUB_TOKEN" ]; then
    # echo "Using GITHUB_TOKEN from environment"
    USE_GITHUB_TOKEN="$GITHUB_TOKEN"
  fi

  # run-all.sh calls test_init() once
  # before calling the test sub-scripts.
  #
  # If caller is not run-all.sh (the subscript
  # is called directly on the command line), then
  # the test_init() is called from here instead.
  #
  if [ "$_SKIP_INIT" = "false" ]; then
    test_init
  else
    # If caller says to skip init, then
    # just assume it was already done.
    TEST_INIT_CALLED=true
  fi

  # Sanity check.
  if [ "$TEST_INIT_CALLED" = "false" ]; then
    fail "test_init() not called"
  fi
}

test_setup_on_sourcing "$@"
