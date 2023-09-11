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

main() {
    local _AT_LEAST_ONE_FATAL_ERROR_FOUND=false
    TEST_SCRIPTS=$(find . -name "*.sh" -type f -not -path "./run-all.sh" -print0 | sort -z | tr '\0' ' ')
    for SCRIPT in $TEST_SCRIPTS; do
        echo "Running $SCRIPT..."
        bash "$SCRIPT"
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

main
