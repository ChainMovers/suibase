#!/bin/bash

# You must source __globals.sh before __dtp-services.sh

DTP_DAEMON_VERSION_FILE="$SUIBASE_BIN_DIR/$DTP_DAEMON_NAME-version.txt"

export DTP_DAEMON_VERSION_INSTALLED=""
update_DTP_DAEMON_VERSION_INSTALLED() {
  # Update the global variable with the version written in the file.
  if [ -f "$DTP_DAEMON_VERSION_FILE" ]; then
    local _FILE_CONTENT
    _FILE_CONTENT=$(cat "$DTP_DAEMON_VERSION_FILE")
    if [ -n "$_FILE_CONTENT" ]; then
      DTP_DAEMON_VERSION_INSTALLED="$_FILE_CONTENT"
      return
    fi
  fi
  DTP_DAEMON_VERSION_INSTALLED=""
}
export -f update_DTP_DAEMON_VERSION_INSTALLED

export DTP_DAEMON_VERSION_SOURCE_CODE=""
update_DTP_DAEMON_VERSION_SOURCE_CODE() {
  # Update the global variable DTP_DAEMON_VERSION_SOURCE_CODE with the version written in the cargo.toml file.
  if [ -f "$DTP_DAEMON_BUILD_DIR/Cargo.toml" ]; then
    local _PARSED_VERSION
    _PARSED_VERSION=$(grep "^version *= *" "$DTP_DAEMON_BUILD_DIR/Cargo.toml" | sed -e 's/version[[:space:]]*=[[:space:]]*"\([0-9]\+\.[0-9]\+\.[0-9]\+\)".*/\1/')
    if [ -n "$_PARSED_VERSION" ]; then
      DTP_DAEMON_VERSION_SOURCE_CODE="$_PARSED_VERSION"
      return
    fi
  fi
  DTP_DAEMON_VERSION_SOURCE_CODE=""
}
export -f update_DTP_DAEMON_VERSION_SOURCE_CODE

need_dtp_daemon_upgrade() {
  # return true if the daemon is not installed, needs to be upgraded
  # or any problem detected.
  #
  # This function assumes that
  #   update_DTP_DAEMON_VERSION_SOURCE_CODE
  #      and
  #   update_DTP_DAEMON_VERSION_INSTALLED
  # have been called before.
  # shellcheck disable=SC2153
  if [ ! -f "$DTP_DAEMON_BIN" ]; then
    true
    return
  fi

  if [ -z "$DTP_DAEMON_VERSION_INSTALLED" ]; then
    true
    return
  fi

  if [ -z "$DTP_DAEMON_VERSION_SOURCE_CODE" ]; then
    true
    return
  fi

  if [ ! "$DTP_DAEMON_VERSION_INSTALLED" = "$DTP_DAEMON_VERSION_SOURCE_CODE" ]; then
    true
    return
  fi

  if [ "${CFG_dtp_enabled:?}" = "dev" ]; then
    # fstat the binaries for differences (target/debug is normally not deleted while "dev").
    local _SRC="$DTP_DAEMON_BUILD_DIR/target/debug/$DTP_DAEMON_NAME"
    local _DST="$DTP_DAEMON_BIN"
    if [ ! -f "$_SRC" ]; then
      true
      return
    fi
    if ! cmp --silent "$_SRC" "$_DST"; then
      echo "$DTP_DAEMON_NAME bin difference detected"
      true
      return
    fi
  fi

  false
  return
}
export -f need_dtp_daemon_upgrade

build_dtp_daemon() {
  #
  # (re)build dtp-services and install it.
  #
  echo "Building $DTP_DAEMON_NAME"
  rm -f "$DTP_DAEMON_VERSION_FILE"
  (if cd "$DTP_DAEMON_BUILD_DIR"; then cargo build -p "$DTP_DAEMON_NAME"; else setup_error "unexpected missing $DTP_DAEMON_BUILD_DIR"; fi)
  # Copy the build result from target to $SUIBASE_BIN_DIR
  local _SRC="$DTP_DAEMON_BUILD_DIR/target/debug/$DTP_DAEMON_NAME"
  if [ ! -f "$_SRC" ]; then
    setup_error "Fail to build $DTP_DAEMON_NAME"
  fi
  # TODO Add a sanity test here before overwriting the installed binary.
  mkdir -p "$SUIBASE_BIN_DIR"
  \cp -f "$_SRC" "$DTP_DAEMON_BIN"
  # Update the installed version file.
  echo "$DTP_DAEMON_VERSION_SOURCE_CODE" >|"$DTP_DAEMON_VERSION_FILE"

  if [ "${CFG_dtp_enabled:?}" != "dev" ]; then
    # Clean the build directory.
    rm -rf "$DTP_DAEMON_BUILD_DIR/target"
  fi
}
export -f build_dtp_daemon

is_dtp_daemon_running() {
  # Quickly determine if the daemon is running, does not check if responding.
  # Has the side effect of updating the DTP_DAEMON_PID variable.
  update_DTP_DAEMON_PID_var

  if [ -n "$DTP_DAEMON_PID" ]; then
    true
    return
  fi

  false
  return
}

start_dtp_daemon() {
  # success/failure is reflected by the DTP_DAEMON_PID var.
  # noop if the process is already started.
  if is_dtp_daemon_running; then
    return
  fi

  # MacOs does not have flock normally installed.
  # If missing, then try to install it.
  update_HOST_vars
  if [ "$HOST_PLATFORM" = "Darwin" ]; then
    if ! which flock >/dev/null 2>&1; then
      if which brew >/dev/null 2>&1; then
        brew install flock >/dev/null 2>&1
      fi
      if ! which flock >/dev/null 2>&1; then
        setup_error "Must install flock. Try 'brew install flock'"
      fi
    fi
  fi

  echo "Starting $DTP_DAEMON_NAME"

  if [ "${CFG_dtp_enabled:?}" = "dev" ]; then
    # Run it in the foreground and just exit when done.
    "$HOME"/suibase/scripts/common/run-daemon.sh dtp foreground
    exit
  fi

  # Try until can confirm the dtp-services is running healthy, or exit
  # if takes too much time.
  end=$((SECONDS + 30))
  ALIVE=false
  AT_LEAST_ONE_SECOND=false
  for _i in {1..3}; do
    # Try to start a script that keeps alive the dtp-services.
    #
    # Will not try if there is already another instance running.
    #
    # run-daemon.sh is design to be flock protected and be silent.
    # All errors will be visible through the dtp-services own logs or by observing
    # which PID owns the flock file. So all output of the script (if any) can
    # safely be ignored to /dev/null.
    nohup "$HOME/suibase/scripts/common/run-daemon.sh" dtp >/dev/null 2>&1 &

    while [ $SECONDS -lt $end ]; do
      if is_dtp_daemon_running; then
        ALIVE=true
        break
      else
        echo -n "."
        sleep 1
        AT_LEAST_ONE_SECOND=true
      fi
      # Detect if should do a retry at starting it.
      # This is unlikely needed, but just in case there is a bug in
      # the startup logic...
      if [ $SECONDS -gt $((SECONDS + 10)) ]; then
        break
      fi
    done

    # If it is alive, then break the retry loop.
    if [ "$ALIVE" = true ]; then
      break
    fi
  done

  # Just UI aesthetic newline for when there was "." printed.
  if [ "$AT_LEAST_ONE_SECOND" = true ]; then
    echo
  fi

  # Act on success/failure of the process responding.
  if [ "$ALIVE" = false ]; then
    if [ ! -f "$DTP_DAEMON_BIN" ]; then
      echo "$DTP_DAEMON_NAME binary not found (build failed?)"
    else
      echo "$DTP_DAEMON_NAME not responding. Try again? (may be the host is too slow?)."
    fi
    exit 1
  fi

  echo "$DTP_DAEMON_NAME started (process pid $DTP_DAEMON_PID)"
}
export -f start_dtp_daemon

restart_dtp_daemon() {
  update_DTP_DAEMON_PID_var
  if [ -z "$DTP_DAEMON_PID" ]; then
    start_dtp_daemon
  else
    # This is a clean restart of the daemon (Ctrl-C). Should be fast.
    kill -s SIGTERM "$DTP_DAEMON_PID"
    echo -n "Restarting $DTP_DAEMON_NAME"
    local _OLD_PID="$DTP_DAEMON_PID"
    end=$((SECONDS + 30))
    while [ $SECONDS -lt $end ]; do
      sleep 1
      update_DTP_DAEMON_PID_var
      if [ -n "$DTP_DAEMON_PID" ] && [ "$DTP_DAEMON_PID" != "$_OLD_PID" ]; then
        echo " (new pid $DTP_DAEMON_PID)"
        return
      fi
    done
    echo " (failed for pid $DTP_DAEMON_PID)"
  fi
}
export -f restart_dtp_daemon

stop_dtp_daemon() {
  # TODO currently unused. Revisit if needed versus a self-exit design.

  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_DTP_DAEMON_PID_var
  if [ -n "$DTP_DAEMON_PID" ]; then
    echo "Stopping $DTP_DAEMON_NAME (process pid $DTP_DAEMON_PID)"

    # TODO This will just restart the daemon... need to actually kill the parents as well!
    kill -s SIGTERM "$DTP_DAEMON_PID"

    # Make sure it is dead.
    end=$((SECONDS + 15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_DTP_DAEMON__PID_var
      if [ -z "$DTP_DAEMON__PID" ]; then
        break
      else
        echo -n "."
        sleep 1
        AT_LEAST_ONE_SECOND=true
      fi
    done

    # Just UI aesthetic newline for when there was "." printed.
    if [ "$AT_LEAST_ONE_SECOND" = true ]; then
      echo
    fi

    if [ -n "$DTP_DAEMON__PID" ]; then
      setup_error " $DTP_DAEMON_NAME pid=$DTP_DAEMON__PID still running. Try again, or stop (kill) the process yourself before proceeding."
    fi
  fi
}
export -f stop_dtp_daemon

export DTP_DAEMON_VERSION=""
update_DTP_DAEMON_VERSION_var() {
  # This will be to get the version from the *running* daemon.
  # --version not supported yet. Always mock it for now.
  DTP_DAEMON_VERSION="0.0.0"
}
export -f update_DTP_DAEMON_VERSION_var

export DTP_DAEMON_PID=""
update_DTP_DAEMON_PID_var() {

  local _PID
  _PID=$(lsof -t -a -c dtp "$SUIBASE_TMP_DIR"/"$DTP_DAEMON_NAME".lock 2>/dev/null)

  #shellcheck disable=SC2181
  if [ $? -eq 0 ]; then
    # Verify _PID is a number. Otherwise, assume it is an error.
    if [[ "$_PID" =~ ^[0-9]+$ ]]; then
      export DTP_DAEMON_PID="$_PID"
      return
    fi
  fi

  # For all error case.

  # Check if lsof is not installed, then inform the user to install it.
  if ! is_installed lsof; then
    setup_error "The CLI command 'lsof' must be installed to use Suibase"
  fi

  DTP_DAEMON_PID=""
}
export -f update_DTP_DAEMON_PID_var

update_dtp_daemon_as_needed() {
  start_dtp_daemon_as_needed "force-update"
}
export -f update_dtp_daemon_as_needed

start_dtp_daemon_as_needed() {

  # When _UPGRADE_ONLY=true:
  #  - Always check to upgrade the daemon.
  #  - Do not start, but restart if *already* running.
  # else:
  #  - if 'dtp_enabled' is true, then check to
  #    upgrade and (re)starts.
  #
  local _UPGRADE_ONLY
  local _SUPPORT_DAEMON
  if [ "$1" = "force-update" ]; then
    _UPGRADE_ONLY=true
    _SUPPORT_DAEMON=true
  else
    # Verify from suibase.yaml if the dtp daemon should be started.
    _UPGRADE_ONLY=false
    if [ "${CFG_dtp_enabled:?}" = "false" ]; then
      _SUPPORT_DAEMON=false
    else
      _SUPPORT_DAEMON=true
    fi
  fi

  # Return 0 on success or not needed.

  if [ "$_SUPPORT_DAEMON" = true ]; then
    update_DTP_DAEMON_VERSION_INSTALLED
    update_DTP_DAEMON_VERSION_SOURCE_CODE
    #echo DTP_DAEMON_VERSION_INSTALLED="$DTP_DAEMON_VERSION_INSTALLED"
    #echo DTP_DAEMON_VERSION_SOURCE_CODE="$DTP_DAEMON_VERSION_SOURCE_CODE"
    if need_dtp_daemon_upgrade; then
      local _OLD_VERSION=$DTP_DAEMON_VERSION_INSTALLED
      build_dtp_daemon
      update_DTP_DAEMON_VERSION_INSTALLED
      local _NEW_VERSION=$DTP_DAEMON_VERSION_INSTALLED
      if [ "$_OLD_VERSION" != "$_NEW_VERSION" ]; then
        if [ -n "$_OLD_VERSION" ]; then
          echo "$DTP_DAEMON_NAME upgraded from $_OLD_VERSION to $_NEW_VERSION"
        fi
      fi
      update_DTP_DAEMON_PID_var
      if [ "$_UPGRADE_ONLY" = true ]; then
        # Restart only if already running.
        if [ -n "$DTP_DAEMON_PID" ]; then
          restart_dtp_daemon
        fi
      else
        # (re)start
        if [ -z "$DTP_DAEMON_PID" ]; then
          start_dtp_daemon
        else
          # Needed for the upgrade to take effect.
          restart_dtp_daemon
        fi
      fi
    else
      if [ -z "$DTP_DAEMON_PID" ] && [ "$_UPGRADE_ONLY" = false ]; then
        # There was no upgrade, but the process need to be started.
        start_dtp_daemon
      fi
    fi
  fi

  # The caller decide what to do if failed.
  if [ "$_SUPPORT_DAEMON" = true ] && [ -z "$DTP_DAEMON_PID" ]; then
    return 1
  fi

  return 0
}
export -f start_dtp_daemon_as_needed

# The response is written in global JSON_RESP
get_dtp_daemon_status() {
  local _DISP=$1 # one of "data", "debug" or "display"

  local _HEADERS="Content-Type: application/json"

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"getLinks\",\"params\":{\"workdir\":\"$WORKDIR_NAME\",\"$_DISP\":true}}"

  JSON_RESP=$(curl -x "" -s --location -X POST "http://${CFG_dtp_host_ip:?}:${CFG_dtp_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS")
}
export -f get_dtp_daemon_status

notify_dtp_daemon_fs_change() {
  # Best-effort notification to the dtp-services that a state/config changed on
  # the filesystem for that workdir. Purposely leave the "path" parameter vague
  # so that the daemon can audit/check *everything*.

  if ! is_dtp_daemon_running; then
    return
  fi

  if [ -z "$WORKDIR_NAME" ]; then
    return
  fi

  local _HEADERS="Content-Type: application/json"

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"fsChange\",\"params\":{\"path\":\"$WORKDIR_NAME\"}}"

  curl --max-time 5 -x "" -s --location -X POST "http://${CFG_dtp_host_ip:?}:${CFG_dtp_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS" >/dev/null 2>&1 &
}
export -f notify_dtp_daemon_fs_change
