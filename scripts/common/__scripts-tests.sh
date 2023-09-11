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
WORKDIRS="$HOME/suibase/workdirs"
OUT="$HOME/suibase/scripts/tests/result.txt"

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

test_setup() {
  # Parse command-line
  #GITHUB_OPTION=false
  #while [[ "$#" -gt 0 ]]; do
  #  case $1 in
  # -t|--target) target="$2"; shift ;; That's an example with a parameter
  # -f|--flag) flag=1 ;; That's an example flag
  #      --github) GITHUB_OPTION=true ;;
  #      *)
  #      fail "Unknown parameter passed: $1";
  #  esac
  #  shift
  #done

  # Clean-up from potential previous execution.
  rm -rf "$OUT"

  # This script should not be called from under workdirs since it will get deleted.
  local _USER_CWD
  _USER_CWD=$(pwd -P)
  if [[ "$_USER_CWD" = *"suibase/workdirs"* ]]; then
    fail "Should not call this test from a directory that will get deleted [suibase/workdirs]"
  fi

  # Add here tests done on github.

  #if [ "$GITHUB_OPTION" = true ]; then
  # Success on github if reaching here.
  #  echo "Test Completed (github early exit)"
  #  exit 0
  #fi

  # Add here tests not run on github.
  #echo "Test Completed"
}

test_setup
