#!/bin/bash

# Utility script to get walrus relay local port from config
# Usage: get-walrus-relay-local-port.sh <workdir>
# Fails if port is not configured (no defaults)

# Check for workdir parameter
if [ $# -ne 1 ]; then
    echo "Usage: $0 <workdir>" >&2
    echo "Example: $0 testnet" >&2
    exit 1
fi

WORKDIR="$1"
SUIBASE_DIR="$HOME/suibase"
SCRIPT_COMMON_CALLER="$0"

# shellcheck source=SCRIPTDIR/../../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# Check if walrus_relay_local_port is configured
if [ -n "${CFG_walrus_relay_local_port:-}" ]; then
    echo "$CFG_walrus_relay_local_port"
    exit 0
fi

# No default - configuration required
echo "Error: walrus_relay_local_port not configured for workdir '$WORKDIR'" >&2
exit 1