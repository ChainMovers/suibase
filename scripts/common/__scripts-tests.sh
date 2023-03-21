#!/bin/bash

# Script to perform sui-base tests.
#
# This is not intended to be called directly by the user.
#
# Use --github when called from a "github action" to limit what
# can be tested in practice on a free tier.
#

# Parse command-line
GITHUB_OPTION=false
while [[ "$#" -gt 0 ]]; do
    case $1 in
        # -t|--target) target="$2"; shift ;; That's an example with a parameter
        # -f|--flag) flag=1 ;; That's an example flag
        --github) GITHUB_OPTION=true ;;
        *)
        echo "Unknown parameter passed: $1";
        exit 1 ;;
    esac
    shift
done

# shellcheck source=SCRIPTDIR/../../../sui-base/install
source ~/sui-base/install
install_ret_code=$?

if [ "$GITHUB_OPTION" = true ]; then
  echo "Installation status code = [$install_ret_code]"
  exit $install_ret_code
fi

# Eventually add more tests here...