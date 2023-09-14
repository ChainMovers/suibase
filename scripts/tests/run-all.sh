#!/bin/bash

# Every test script (.sh files) in this directory and subdirectories must succeed
# to pass on github commit.
#
# Run all tests with '~/suibase/tests/run-all.sh'
# Calls are done in alphabetical order.
# Alternatively, you can manually run each test script for debugging.
#
# A test script returns one of the following value:
#  0 No error found
#  1 At least one fatal error found. No point to do further tests. The code should
#    not be deployed.
#  2 Test skipped

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"
test_init

# As needed, create scripts/templates/common/suibase.yaml
init_common_template

# Note: Do not load globals.sh here. It will be loaded by each test script.

main() {

  # Validate command-line.
  #
  # By default the tests are extensive and can take >1hour.
  #
  # 2 options (can be combined):
  #   --fast: Intended to validate quickly. Just a few sanity tests. Goal is <5 minutes.
  #   --main_branch: Tests using main branch of Mysten Labs. For "on the edge" validation.
  #
  local _PASSTHRU_OPTIONS=()
  while [[ "$#" -gt 0 ]]; do
    case $1 in
    #-t|--target) target="$2"; shift ;; That's an example with a parameter
    # -f|--flag) flag=1 ;; That's an example flag
    --fast)
      _PASSTHRU_OPTIONS+=("$1")
      ;;
    --main_branch)
      _PASSTHRU_OPTIONS+=("$1")
      ;;
    --github_token)
      _PASSTHRU_OPTIONS+=("$1")
      _PASSTHRU_OPTIONS+=("$2")
      shift
      ;;
    *)
      fail "Unknown parameter passed: $1"
      ;;
    esac
    shift
  done

  local _AT_LEAST_ONE_FATAL_ERROR_FOUND=false
  local _ALL_FILES
  _ALL_FILES=$(find ~/suibase/scripts/tests/. -name "*.sh" -type f -print0 | sort -z | tr '\0' ' ')
  local _ERROR_COUNT=0
  local _SKIP_COUNT=0
  local _PASS_COUNT=0
  local _EXECUTED_COUNT=0

  local _TEST_SCRIPTS_TO_RUN=()
  for _SCRIPT_FILEPATH in $_ALL_FILES; do
    # Get the filename from $SCRIPT
    local _SCRIPT_NAME
    _SCRIPT_NAME=$(basename "$_SCRIPT_FILEPATH")

    # Skip run-all.sh!
    if [[ "$_SCRIPT_NAME" == "run-all.sh" ]]; then
      continue
    fi

    # Skip libraries (always starting with double underscore).
    if [[ "$_SCRIPT_NAME" == "__"* ]]; then
      continue
    fi

    _TEST_SCRIPTS_TO_RUN+=("$_SCRIPT_FILEPATH")
  done

  for _SCRIPT_FILEPATH in "${_TEST_SCRIPTS_TO_RUN[@]}"; do
    # Get the filename from $SCRIPT
    local _SCRIPT_NAME
    _SCRIPT_NAME=$(basename "$_SCRIPT_FILEPATH")

    _EXECUTED_COUNT=$((_EXECUTED_COUNT + 1))
    echo "Running $_SCRIPT_FILEPATH..."
    ("$_SCRIPT_FILEPATH" "${_PASSTHRU_OPTIONS[@]}" --skip_init)
    local _CODE=$?
    case $_CODE in
    0)
      _PASS_COUNT=$((_PASS_COUNT + 1))
      ;;
    2)
      _SKIP_COUNT=$((_SKIP_COUNT + 1))
      echo "Skipped $_SCRIPT_NAME."
      ;;
    *)
      _ERROR_COUNT=$((_ERROR_COUNT + 1))
      echo "Error code=$_CODE from $_SCRIPT_NAME."
      if [[ $_CODE -eq 1 ]]; then
        break
      fi
      ;;
    esac
  done

  local _TEST_SCRIPTS_TO_RUN_SIZE=${#_TEST_SCRIPTS_TO_RUN[@]}
  local _NOT_EXECUTED_COUNT=$((_TEST_SCRIPTS_TO_RUN_SIZE - _EXECUTED_COUNT))

  printf "\nSummary\n"
  printf "=======\n"
  if [ "$_ERROR_COUNT" -gt 0 ]; then
    printf "Failed : \033[1;31m%3d\033[0m\n" "$_ERROR_COUNT"
  else
    printf "Failed : %3d\n" "$_ERROR_COUNT"
  fi
  printf "Skipped: %3d\n" "$_SKIP_COUNT"
  printf "Not run: %3d\n" "$_NOT_EXECUTED_COUNT"
  printf "Passed : %3d\n" "$_PASS_COUNT"
  printf "        ____\n"
  printf "Total :  %3d\n\n" "$_TEST_SCRIPTS_TO_RUN_SIZE"
  if [ "$_ERROR_COUNT" -gt 0 ]; then
    printf "\033[1;31mError\033[0m : Test Failed. Code should not be deployed.\n"
    exit 1
  else
    printf "\033[1;32mSuccess\033[0m : No problems found.\n"
    exit 0
  fi
}

main "$@"
