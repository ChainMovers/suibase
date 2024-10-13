#!/bin/bash

# This is not intended to be called by the user directly. It is used by the suibase scripts
# to start the execution of the suibase-daemon and dtp-services
#
# Does also:
#  - Helps prevent running multiple instance of a daemon at the same time.
#  - Restart the daemon on exit/panic (after a brief delay).
#
# Why not systemd? Portability. If the host can run bash, then this works.
#
# First parameter can be one of "suibase" or "dtp"
# Second parameter can be optionally "foreground" for debug purpose.
PARAM_NAME="$1"
PARAM_CMD="$2"

# Source '__globals.sh'.
SUIBASE_DIR="$HOME/suibase"
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"

# shellcheck source=SCRIPTDIR/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/__apps.sh
source "$SUIBASE_DIR/scripts/common/__apps.sh"

# Switch case on $1 being "suibase" or "dtp".
case "$PARAM_NAME" in
"suibase")
  # shellcheck source=SCRIPTDIR/__suibase-daemon.sh
  source "$SUIBASE_DIR/scripts/common/__suibase-daemon.sh"
  # shellcheck source=SCRIPTDIR/__sui-faucet-process.sh
  source "$SUIBASE_DIR/scripts/common/__sui-faucet-process.sh"
  ;;
"dtp")
  # shellcheck source=SCRIPTDIR/__dtp-daemon.sh
  source "$SUIBASE_DIR/scripts/common/__dtp-daemon.sh"
  ;;
*)
  echo "ERROR: Invalid daemon name: $PARAM_NAME"
  exit 1
  ;;
esac


force_stop_all_services() {
  # This will force stop all processes for localnet, but not touch
  # the "user_request" to preserve the user intent.
  #
  # It will create a "force_stop" key-value in each .state workdir
  # to indicate the temporary need to stop the services (help
  # debugging?)
  #
  # It is assumed that the daemon will delete the force_stop,
  # and resume the services as configured by "user_request".

  update_SUI_FAUCET_PROCESS_PID_var
  if [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
    set_key_value "localnet" "force_stop" "true"
    stop_sui_faucet_process
  fi

  update_SUI_PROCESS_PID_var
  if [ -n "$SUI_PROCESS_PID" ]; then
    set_key_value "localnet" "force_stop" "true"
    stop_sui_process
  fi

  # Wait until all process confirm stopped (or timeout).
  count=0
  while [ $count -lt 10 ]; do
    if is_suibase_daemon_running; then
      # Already running? then do nothing.
      exit 0
    fi
    update_SUI_FAUCET_PROCESS_PID_var
    update_SUI_PROCESS_PID_var
    if [ -z "$SUI_FAUCET_PROCESS_PID" ] && [ -z "$SUI_PROCESS_PID" ]; then
      break
    fi
    sleep 1
    count=$((count + 1))
  done
}
export -f force_stop_all_services

main() {
  # Detect if suibase is not installed!
  if [ ! -d "$SUIBASE_DIR" ]; then
    echo "ERROR: suibase is not installed! Check https://suibase.io/how-to/install"
    exit 1
  fi

  local _DAEMON_BIN
  local _DAEMON_NAME
  case "$PARAM_NAME" in
  "suibase")
    _DAEMON_BIN="$SUIBASE_DAEMON_BIN"
    _DAEMON_NAME="$SUIBASE_DAEMON_NAME"
    ;;
  "dtp")
    _DAEMON_BIN="$DTP_DAEMON_BIN"
    _DAEMON_NAME="$DTP_DAEMON_NAME"
    ;;
  *)
    echo "ERROR: Invalid daemon name: $PARAM_NAME"
    exit 1
    ;;
  esac

  if [ ! -f "$_DAEMON_BIN" ]; then
    echo "The $_DAEMON_NAME binary does not exists!"
    exit 1
  fi

  # Run the daemon from a script that constantly restart it on
  # abnormal exit (e.g. panic).
  mkdir -p "$SUIBASE_LOGS_DIR"
  mkdir -p "$SUIBASE_TMP_DIR"

  local _LOCKFILE="$SUIBASE_TMP_DIR/$_DAEMON_NAME.lock"
  local _LOG="$SUIBASE_LOGS_DIR/$_DAEMON_NAME.log"
  local _CMD_LINE="$_DAEMON_BIN run"

  if [ "$PARAM_CMD" == "foreground" ]; then
    # Run in foreground, with no restart on exit/panic.
    # shellcheck disable=SC2086,SC2016
    try_locked_command "$_LOCKFILE" /bin/sh -uec '"$@" 2>&1 | tee -a $0' "$_LOG" $_CMD_LINE
  else
    # Run in background, with auto-restart on exit/panic.
    echo "Starting $_DAEMON_NAME in background. Check logs at: $_LOG"
    echo "$_CMD_LINE"

    if [ "$PARAM_CMD" == "cli-call" ]; then
      # Subsequent cli_mutex_ calls are NOOP because the script
      # was called by a script already holding the proper locks.
      cli_mutex_disable
    fi

    # Detect scenario where the suibase-daemon is not running and
    # the lockfile is still present.
    #
    # This can happen in some scenario where both the daemon and its restart
    # loop was killed.
    #
    # In this scenario, when child process were started by the daemon and are still running
    # (e.g localnet sui and sui-faucet) it is not possible to recover the restart mechanism
    # because of the lock.
    #
    # Therefore, this script will stop the child processes which will remove the lockfile.
    #
    # The suibase-daemon is responsible to properly re-start the child processes/services.
    if [ "$PARAM_NAME" = "suibase" ]; then
      cli_mutex_lock "suibase_daemon"
      if [ -f "$_LOCKFILE" ]; then
        for i in 1 2 3; do
          if is_suibase_daemon_running; then
            # Already running? then do nothing.
            exit 0
          fi

          if ! [ -f "$_LOCKFILE" ]; then
            break
          fi

          # This is to recover when the lockfile exists, but the suibase-daemon
          # is NOT running (e.g. was killed). Killing only the daemon is problematic
          # when its child process are left running. This is why all potential
          # child services are stopped here.
          force_stop_all_services
        done
      fi
      # Must release here, because this process might "never" exit and clean-up from trap.
      cli_mutex_release "suibase_daemon"
    fi

    # shellcheck disable=SC2086,SC2016
    try_locked_command "$_LOCKFILE" /bin/sh -uec '
    while true; do
      "$@" > $0 2>&1
      exit_status=$?
      if [ $exit_status -eq 13 ]; then
        echo "Process exited with status 13. Exiting loop." > $0 2>&1
        break
      fi
      echo "Restarting process" > $0 2>&1
      sleep 1
    done' "$_LOG" $_CMD_LINE
  fi

}

main "$@"
