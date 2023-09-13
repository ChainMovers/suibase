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
#  2 At least one error found, but further tests could proceed. The code
#    should not be deployed.

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../common/__scripts-tests.sh
source "$SUIBASE_DIR/scripts/common/__scripts-tests.sh"

main() {
  test_init

  # Parse command-line.
  #
  # By default the tests are extensive and can take >1hour.
  #
  # 2 options (can be combined):
  #   --fast: Intended to validate quickly. Just a few sanity tests. Goal is <5 minutes.
  #   --main_branch: Tests using main branch of Mysten Labs. For "on the edge" validation.
  #
  local _PASSTHRU_OPTIONS=()
  local _GITHUB_TOKEN=""
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
  TEST_SCRIPTS=$(find ~/suibase/scripts/tests/. -name "*.sh" -type f -print0 | sort -z | tr '\0' ' ')
  for SCRIPT in $TEST_SCRIPTS; do
    # Skip run-all.sh!
    if [[ "$SCRIPT" == *"run-all.sh" ]]; then
      continue
    fi
    echo "Running $SCRIPT..."
    ("$SCRIPT" "${_PASSTHRU_OPTIONS[@]}" --skip_init)
    local _CODE=$?
    if [[ $_CODE -ne 0 ]]; then
      _AT_LEAST_ONE_FATAL_ERROR_FOUND=true
      echo "Fatal error found in $SCRIPT. Exit code=$_CODE"
      if [[ $_CODE -eq 1 ]]; then
        break
      fi
    fi
  done

  if [ "$_AT_LEAST_ONE_FATAL_ERROR_FOUND" = "true" ]; then
    echo "Fatal error found. Code should not be deployed."
    exit 1
  else
    echo "All tests passed."
    exit 0
  fi
}

main "$@"
