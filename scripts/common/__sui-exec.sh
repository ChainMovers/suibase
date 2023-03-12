#!/bin/bash

# This script simply call the proper sui binary and config combination.
#
# You use 'sui-exec' in the same way you would use 'sui' from Mysten. Example:
#    'sui-exec client gas'
#
# One convenience is you do not have to specify the --client.config,
# , --network.config and --keystore-path options on the command line.
#
# Never move this script outside of its workdir. It must stay
# here to run within its context.

# The workdir name is the directory name of *this* script location.

# Source 'sui-base/common/__globals.sh'.
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="$(basename $(dirname "$SCRIPT_COMMON_CALLER"))"
source "$HOME/sui-base/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

sui_exec "$@"
