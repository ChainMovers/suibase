#!/bin/bash

# This is not intended to be called by the user directly. It is used by the suibase scripts
# to start the execution of the suibase-daemon.
#
# Does also:
#  - Helps prevent running multiple instance of the daemon at the same time.
#  - Restart the daemon on exit/panic (after a brief delay).
#
# Why not systemd? Portability. If the host can run bash, then this works.
#

# Source '__globals.sh'.
SUIBASE_DIR="$HOME/suibase"
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
# shellcheck source=SCRIPTDIR/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

# shellcheck source=SCRIPTDIR/__suibase-daemon.sh
source "$SUIBASE_DIR/scripts/common/__suibase-daemon.sh"

# Check what is available, prefer flock over lockf.
# Reference: https://github.com/Freaky/run-one/blob/master/run-one
if is_installed flock; then
  _LOCK_CMD="flock -xn"
else
  if is_installed lockf; then
    _LOCK_CMD="lockf -st0"
  else
    setup_error "Neither 'flock' or 'lockf' are available! Install one of them"
  fi
fi

locked_command() {
  exec $_LOCK_CMD "$@"
}

main() {
  # Only command supported is "foreground" for special execution
  # when developing/debugging.
  local _CMD="$1"

  # Detect if suibase is not installed!
  if [ ! -d "$SUIBASE_DIR" ]; then
    echo "ERROR: suibase is not installed! Check https://suibase.io/how-to/install"
    exit 1
  fi

  if [ ! -f "$SUIBASE_DAEMON_BIN" ]; then
    echo "The $SUIBASE_DAEMON_NAME binary does not exists!"
    exit 1
  fi

  # Run the daemon from a script that constantly restart it on
  # abnormal exit (e.g. panic).
  mkdir -p "$SUIBASE_LOGS_DIR"
  mkdir -p "$SUIBASE_TMP_DIR"

  local _LOCKFILE="$SUIBASE_TMP_DIR/$SUIBASE_DAEMON_NAME.lock"
  local _LOG="$SUIBASE_LOGS_DIR/$SUIBASE_DAEMON_NAME.log"
  local _CMD_LINE="$SUIBASE_DAEMON_BIN run"

  if [ "$_CMD" == "foreground" ]; then
    # Run in foreground, with no restart on exit/panic.
    # shellcheck disable=SC2086,SC2016
    locked_command "$_LOCKFILE" /bin/sh -uec '"$@" 2>&1 | tee -a $0' "$_LOG" $_CMD_LINE
  else
    # Run in background, with auto-restart on exit/panic.
    # shellcheck disable=SC2086,SC2016
    locked_command "$_LOCKFILE" /bin/sh -uec 'while true; do "$@" > $0 2>&1; echo "Restarting process" > $0 2>&1; sleep 1; done' "$_LOG" $_CMD_LINE
  fi

}

main "$@"
