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

test_init() {
  if [ "$TEST_INIT_CALLED" = "true" ]; then
    return
  fi
  TEST_INIT_CALLED=true
  rm -rf "$OUT"
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
    if ! grep -q "github_token:" "$HOME/suibase/scripts/templates/common/suibase.yaml"; then
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

test_setup_on_sourcing() {
  # Parse command-line
  FAST_OPTION=false
  MAIN_BRANCH_OPTION=false
  local _SKIP_INIT=false
  while [[ "$#" -gt 0 ]]; do
    case $1 in
    --fast) FAST_OPTION=true ;;
    --main_branch) MAIN_BRANCH_OPTION=true ;;
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
    # jsut assume it was already done.
    TEST_INIT_CALLED=true
  fi

  # Sanity check.
  if [ "$TEST_INIT_CALLED" = "false" ]; then
    fail "test_init() not called"
  fi
}

test_setup_on_sourcing "$@"
