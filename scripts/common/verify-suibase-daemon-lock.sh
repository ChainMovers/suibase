#!/bin/bash

# Source '__globals.sh'.
SUIBASE_DIR="$HOME/suibase"
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="none"

# Validate that the parameter is a number
if [ -z "$1" ]; then
  echo "ERROR: Missing PID to check"
  exit 1
fi

if ! [[ "$1" =~ ^[0-9]+$ ]]; then
  echo "ERROR: Invalid PID: $1"
  exit 1
fi

# shellcheck source=SCRIPTDIR/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/__suibase-daemon.sh
source "$SUIBASE_DIR/scripts/common/__suibase-daemon.sh"

# Get the PID of suibase (under the lock file)
update_SUIBASE_DAEMON_PID_var

if [ -z "$SUIBASE_DAEMON_PID" ]; then
  echo "ERROR: suibase daemon not running under lock"
  exit 1
fi

if [ "$1" != "$SUIBASE_DAEMON_PID" ]; then
  echo "ERROR: PID $1 is not the suibase-daemon PID under proper lock ($SUIBASE_DAEMON_PID)"
  exit 1
fi

echo "OK"
exit 0
