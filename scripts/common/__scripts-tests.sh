#!/bin/bash

# Script to perform suibase tests.
#
# This is not intended to be called directly by the user.
#
# When something is wrong ideally do:
#       fail "any string"
#
#          or
#
#      "exit" with non-zero.
#
export WORKDIRS="$HOME/suibase/workdirs"
export OUT="$HOME/suibase/scripts/tests/result.txt"

# Initialized by test_setup()
export FAST_OPTION=false
export MAIN_BRANCH_OPTION=false
export TEST_INIT_CALLED=false

# List of workdir routinely tested.
export WORKDIRS_NAME=(
  "localnet"
  "devnet"
  "testnet"
  "mainnet"
)

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

assert_workdir_ok() {
  _DIR="$WORKDIRS/$1"

  # Verify minimal integrity of workdirs/_DIR.
  [ -d "$WORKDIRS" ] || fail "workdirs missing"
  [ -L "$WORKDIRS/active" ] || fail "workdirs/active missing"
  [ -d "$_DIR" ] || fail "workdirs/localnet missing"

  [ -f "$_DIR/sui-exec" ] || fail "workdirs/sui-exec missing"
  [ -x "$_DIR/sui-exec" ] || fail "workdirs/sui-exec not exec"

  [ -f "$_DIR/workdir-exec" ] || fail "workdirs/workdir-exec missing"
  [ -x "$_DIR/workdir-exec" ] || fail "workdirs/workdir-exec not exec"

  [ -f "$_DIR/suibase.yaml" ] || fail "workdirs/suibase.yaml missing"

  # First word out of "workdir-exec" should be the workdir name
  local _HELP
  _HELP=$("$_DIR/workdir-exec")
  _RESULT="$?"
  if [ ! "$_RESULT" -eq 0 ]; then
    fail "workdir-exec usage should not be an error"
  fi
  _FIRST_WORD=$(echo "$_HELP" | head -n1 | awk '{print $1;}')
  # Note: Must use contain because of the ANSI color escape code.
  [[ "$_FIRST_WORD" == *"$1"* ]] || fail "usage first word [$_FIRST_WORD] not [$1]"

  # Usage should have the suibase version, so sanity verify for "suibase"
  [[ "$_HELP" == *"suibase"* ]] || fail "usage does not mention suibase [$_HELP]"
}

assert_build_ok() {
  local _SUI_BIN="$WORKDIRS/$1/sui-repo/target/debug/sui"
  # Verify that the sui-repo and the binary are OK.
  local _VERSION _FIRST_WORD
  _VERSION=$($_SUI_BIN --version)
  _FIRST_WORD=$(echo "$_VERSION" | head -n1 | awk '{print $1;}')
  [ "$_FIRST_WORD" = "sui" ] || fail "sui --version did not work [$_VERSION]"
}

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

test_setup() {

  # Parse command-line
  FAST_OPTION=false
  MAIN_BRANCH_OPTION=false
  local _SKIP_INIT=false
  while [[ "$#" -gt 0 ]]; do
    case $1 in
    --fast) FAST_OPTION=true ;;
    --main_branch) MAIN_BRANCH_OPTION=true ;;
    --skip_init) _SKIP_INIT=true ;;
    *)
      fail "Unknown parameter passed: $1"
      ;;
    esac
    shift
  done

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
export -f test_setup
