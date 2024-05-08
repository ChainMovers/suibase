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

force_stop_all_services() {
  # This is called in rare cases.
  #
  # It will force stop all processes for all workdirs, but not touch
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
    locked_command "$_LOCKFILE" /bin/sh -uec '"$@" 2>&1 | tee -a $0' "$_LOG" $_CMD_LINE
  else
    # Run in background, with auto-restart on exit/panic.
    echo "Starting $_DAEMON_NAME in background. Check logs at: $_LOG"
    echo "$_CMD_LINE"

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
      if [ -f "$_LOCKFILE" ]; then
        local _BLOCKED=true
        for i in 1 2 3; do
          if is_suibase_daemon_running; then
            _BLOCKED=false
            break
          fi

          if ! [ -f "$_LOCKFILE" ]; then
            _BLOCKED=false
            break
          fi

          sleep 1
        done

        if [ $_BLOCKED = true ]; then
          echo "WARNING: $_DAEMON_NAME lock exists and the daemon is not running. Stopping all services."
          force_stop_all_services
        fi
      fi
      # shellcheck disable=SC2086,SC2016
      locked_command "$_LOCKFILE" /bin/sh -uec 'while true; do "$@" > $0 2>&1; echo "Restarting process" > $0 2>&1; sleep 1; done' "$_LOG" $_CMD_LINE
    fi
  fi

}

main "$@"
