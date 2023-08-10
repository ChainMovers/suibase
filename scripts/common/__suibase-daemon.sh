#!/bin/bash

# You must source __globals.sh before __suibase-daemon.sh

SUIBASE_DAEMON_VERSION_FILE="$SUIBASE_BIN_DIR/$SUIBASE_DAEMON_NAME-version.txt"

export SUIBASE_DAEMON_VERSION_INSTALLED=""
update_SUIBASE_DAEMON_VERSION_INSTALLED() {
  # Update the global variable with the version written in the file.
  if [ -f "$SUIBASE_DAEMON_VERSION_FILE" ]; then
    local _FILE_CONTENT
    _FILE_CONTENT=$(cat "$SUIBASE_DAEMON_VERSION_FILE")
    if [ -n "$_FILE_CONTENT" ]; then
      SUIBASE_DAEMON_VERSION_INSTALLED="$_FILE_CONTENT"
      return
    fi
  fi
  SUIBASE_DAEMON_VERSION_INSTALLED=""
}
export -f update_SUIBASE_DAEMON_VERSION_INSTALLED

export SUIBASE_DAEMON_VERSION_SOURCE_CODE=""
update_SUIBASE_DAEMON_VERSION_SOURCE_CODE() {
  # Update the global variable SUIBASE_DAEMON_VERSION_SOURCE_CODE with the version written in the cargo.toml file.
  if [ -f "$SUIBASE_DAEMON_BUILD_DIR/Cargo.toml" ]; then
    local _PARSED_VERSION
    _PARSED_VERSION=$(grep "^version *= *" "$SUIBASE_DAEMON_BUILD_DIR/Cargo.toml" | sed -e 's/version[[:space:]]*=[[:space:]]*"\([0-9]\+\.[0-9]\+\.[0-9]\+\)".*/\1/')
    if [ -n "$_PARSED_VERSION" ]; then
      SUIBASE_DAEMON_VERSION_SOURCE_CODE="$_PARSED_VERSION"
      return
    fi
  fi
  SUIBASE_DAEMON_VERSION_SOURCE_CODE=""
}
export -f update_SUIBASE_DAEMON_VERSION_SOURCE_CODE

need_suibase_daemon_upgrade() {
  # return true if the daemon is not installed, needs to be upgraded
  # or any problem detected.
  #
  # This function assumes that
  #   update_SUIBASE_DAEMON_VERSION_SOURCE_CODE
  #      and
  #   update_SUIBASE_DAEMON_VERSION_INSTALLED
  # have been called before.

  if [ ! -f "$SUIBASE_DAEMON_BIN" ]; then
    true
    return
  fi

  if [ -z "$SUIBASE_DAEMON_VERSION_INSTALLED" ]; then
    true
    return
  fi

  if [ -z "$SUIBASE_DAEMON_VERSION_SOURCE_CODE" ]; then
    true
    return
  fi

  if [ ! "$_SOURCE_VERSION" = "$_INSTALLED_VERSION" ]; then
    true
    return
  fi

  if [ "${CFG_proxy_enabled:?}" == "dev" ]; then
    # fstat the binaries for differences (the target/debug should not be deleted).
    local _SRC="$SUIBASE_DAEMON_BUILD_DIR/target/debug/$SUIBASE_DAEMON_NAME"
    local _DST="$SUIBASE_DAEMON_BIN"
    if [ ! -f "$_SRC" ]; then
      true
      return
    fi
    if ! cmp -s "$_SRC" "$_DST"; then
      echo "$SUIBASE_DAEMON_NAME bin difference detected"
      true
      return
    fi
  fi

  false
  return
}
export -f need_suibase_daemon_upgrade

build_suibase_daemon() {
  #
  # (re)build suibase-daemon and install it.
  #
  if [ "${CFG_proxy_enabled:?}" == "false" ]; then
    return
  fi

  echo "Building $SUIBASE_DAEMON_NAME"
  rm -f "$SUIBASE_DAEMON_VERSION_FILE"
  (if cd "$SUIBASE_DAEMON_BUILD_DIR"; then cargo build -p "$SUIBASE_DAEMON_NAME"; else setup_error "unexpected missing $SUIBASE_DAEMON_BUILD_DIR"; fi)
  # Copy the build result from target to $SUIBASE_BIN_DIR
  local _SRC="$SUIBASE_DAEMON_BUILD_DIR/target/debug/$SUIBASE_DAEMON_NAME"
  if [ ! -f "$_SRC" ]; then
    setup_error "Fail to build $SUIBASE_DAEMON_NAME"
  fi
  # TODO Add a sanity test here before overwriting the installed binary.
  mkdir -p "$SUIBASE_BIN_DIR"
  \cp -f "$_SRC" "$SUIBASE_DAEMON_BIN"
  # Update the installed version file.
  echo "$SUIBASE_DAEMON_VERSION_SOURCE_CODE" >|"$SUIBASE_DAEMON_VERSION_FILE"

  if [ "${CFG_proxy_enabled:?}" != "dev" ]; then
    # Clean the build directory.
    rm -rf "$SUIBASE_DAEMON_BUILD_DIR/target"
  fi
}
export -f build_suibase_daemon

start_suibase_daemon() {
  # success/failure is reflected by the SUIBASE_DAEMON_PID var.
  # noop if the process is already started.

  if [ "${CFG_proxy_enabled:?}" == "false" ]; then
    return
  fi

  update_SUIBASE_DAEMON_PID_var
  if [ -n "$SUIBASE_DAEMON_PID" ]; then
    return
  fi

  echo "Starting $SUIBASE_DAEMON_NAME"

  if [ "${CFG_proxy_enabled:?}" == "dev" ]; then
    # Run it in the foreground and just exit when done.
    "$HOME/suibase/scripts/common/run-suibase-daemon.sh"
    exit
  fi

  # Try until can confirm the suibase-daemon is running healthy, or exit
  # if takes too much time.
  end=$((SECONDS + 30))
  ALIVE=false
  AT_LEAST_ONE_SECOND=false
  for _i in {1..3}; do
    if $SUI_BASE_NET_MOCK; then
      export SUIBASE_DAEMON_PID=$SUI_BASE_NET_MOCK_PID
    else
      # Try to start a script that keeps alive the suibase-daemon.
      #
      # Will not try if there is already another instance running.
      #
      # run-suibase-daemon.sh is design to be flock protected and be silent.
      # All errors will be visible through the suibase-daemon own logs or by observing
      # which PID owns the flock file. So all output of the script (if any) can
      # safely be ignored to /dev/null.
      nohup "$HOME/suibase/scripts/common/run-suibase-daemon.sh" >/dev/null 2>&1 &
    fi

    while [ $SECONDS -lt $end ]; do
      # If CHECK_ALIVE is anything but empty string... then it is alive!
      if $SUI_BASE_NET_MOCK; then
        CHECK_ALIVE="it's alive!"
      else
        # TODO: Actually implement that the assigned port for this workdir is responding!
        CHECK_ALIVE=$(lsof "$SUIBASE_TMP_DIR"/"$SUIBASE_DAEMON_NAME".lock 2>/dev/null | grep "suibase")
      fi
      if [ -n "$CHECK_ALIVE" ]; then
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
    if [ ! -f "$SUIBASE_DAEMON_BIN" ]; then
      echo "$SUIBASE_DAEMON_NAME binary not found (build failed?)"
    else
      echo "$SUIBASE_DAEMON_NAME not responding. Try again? (may be the host is too slow?)."
    fi
    exit 1
  fi

  update_SUIBASE_DAEMON_PID_var
  echo "$SUIBASE_DAEMON_NAME started (process pid $SUIBASE_DAEMON_PID)"
}
export -f start_suibase_daemon

restart_suibase_daemon() {
  update_SUIBASE_DAEMON_PID_var
  if [ -z "$SUIBASE_DAEMON_PID" ]; then
    start_suibase_daemon
  else
    # This is a clean restart of the daemon (Ctrl-C). Should be fast.
    kill -s SIGTERM "$SUIBASE_DAEMON_PID"
    echo -n "Restarting $SUIBASE_DAEMON_NAME"
    local _OLD_PID="$SUIBASE_DAEMON_PID"
    end=$((SECONDS + 30))
    while [ $SECONDS -lt $end ]; do
      sleep 1
      update_SUIBASE_DAEMON_PID_var
      if [ -n "$SUIBASE_DAEMON_PID" ] && [ "$SUIBASE_DAEMON_PID" != "$_OLD_PID" ]; then
        echo " (new pid $SUIBASE_DAEMON_PID)"
        return
      fi
    done
    echo " (failed for pid $SUIBASE_DAEMON_PID)"
  fi
}
export -f restart_suibase_daemon

stop_suibase_daemon() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUIBASE_DAEMON_PID_var
  if [ -n "$SUIBASE_DAEMON_PID" ]; then
    echo "Stopping $SUIBASE_DAEMON_NAME (process pid $SUIBASE_DAEMON_PID)"

    if $SUI_BASE_NET_MOCK; then
      unset SUIBASE_DAEMON_PID
    else
      # TODO This will just restart the daemon... need to actually kill the parents as well!
      kill -s SIGTERM "$SUIBASE_DAEMON_PID"
    fi

    # Make sure it is dead.
    end=$((SECONDS + 15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_SUIBASE_DAEMON__PID_var
      if [ -z "$SUIBASE_DAEMON__PID" ]; then
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

    if [ -n "$SUIBASE_DAEMON__PID" ]; then
      setup_error " $SUIBASE_DAEMON_NAME pid=$SUIBASE_DAEMON__PID still running. Try again, or stop (kill) the process yourself before proceeding."
    fi
  fi
}
export -f stop_suibase_daemon

export SUIBASE_DAEMON_VERSION=""
update_SUIBASE_DAEMON_VERSION_var() {
  # This will be to get the version from the *running* daemon.
  # --version not supported yet. Always mock it for now.
  SUIBASE_DAEMON_VERSION="0.0.0"
}
export -f update_SUIBASE_DAEMON_VERSION_var

export SUIBASE_DAEMON_PID=""
update_SUIBASE_DAEMON_PID_var() {
  if $SUI_BASE_NET_MOCK; then return; fi

  local _PID
  _PID=$(lsof -t -a -c suibase "$SUIBASE_TMP_DIR"/"$SUIBASE_DAEMON_NAME".lock 2>/dev/null)

  #shellcheck disable=SC2181
  if [ $? -eq 0 ]; then
    # Verify _PID is a number. Otherwise, assume it is an error.
    if [[ "$_PID" =~ ^[0-9]+$ ]]; then
      export SUIBASE_DAEMON_PID="$_PID"
      return
    fi
  fi
  # For all error case.
  SUIBASE_DAEMON_PID=""
}
export -f update_SUIBASE_DAEMON_PID_var

start_suibase_daemon_as_needed() {
  # Return 0 on success or not needed.

  # Verify from suibase.yaml if the suibase daemon should be started.
  local _SUPPORT_PROXY
  if [ "${CFG_proxy_enabled:?}" == "false" ]; then
    _SUPPORT_PROXY=false
  else
    _SUPPORT_PROXY=true
  fi

  if [ "$_SUPPORT_PROXY" = true ]; then
    update_SUIBASE_DAEMON_VERSION_INSTALLED
    update_SUIBASE_DAEMON_VERSION_SOURCE_CODE
    #echo SUIBASE_DAEMON_VERSION_INSTALLED="$SUIBASE_DAEMON_VERSION_INSTALLED"
    #echo SUIBASE_DAEMON_VERSION_SOURCE_CODE="$SUIBASE_DAEMON_VERSION_SOURCE_CODE"
    if need_suibase_daemon_upgrade; then
      local _OLD_VERSION=$SUIBASE_DAEMON_VERSION_INSTALLED
      build_suibase_daemon
      update_SUIBASE_DAEMON_VERSION_INSTALLED
      local _NEW_VERSION=$SUIBASE_DAEMON_VERSION_INSTALLED
      if [ "$_OLD_VERSION" != "$_NEW_VERSION" ]; then
        if [ -n "$_OLD_VERSION" ]; then
          echo "$SUIBASE_DAEMON_NAME upgraded from $_OLD_VERSION to $_NEW_VERSION"
        fi
      fi
      update_SUIBASE_DAEMON_PID_var
      if [ -z "$SUIBASE_DAEMON_PID" ]; then
        start_suibase_daemon
      else
        # Needed for the upgrade to take effect.
        restart_suibase_daemon
      fi
    else
      if [ -z "$SUIBASE_DAEMON_PID" ]; then
        # There was no need for upgrade, but the process need to be started.
        start_suibase_daemon
      fi
    fi
  fi

  # The caller decide what to do if failed.
  if [ "$_SUPPORT_PROXY" = true ] && [ -z "$SUIBASE_DAEMON_PID" ]; then
    return 1
  fi

  return 0
}
export -f start_suibase_daemon_as_needed

# The response is written in global JSON_RESP
get_suibase_daemon_status() {
  local _DISP=$1 # one of "data", "debug" or "display"

  local _HEADERS="Content-Type: application/json"

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"getLinks\",\"params\":{\"workdir\":\"$WORKDIR\",\"$_DISP\":true}}"

  JSON_RESP=$(curl -x "" -s --location -X POST "http://0.0.0.0:${CFG_daemon_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS")
}
export -f get_suibase_daemon_status

show_suibase_daemon_get_links() {
  local _DEBUG_OPTION=$1
  local _JSON_PARAM=$2

  local _USER_REQUEST
  _USER_REQUEST=$(get_key_value "user_request")

  if [ "$_USER_REQUEST" = "stop" ]; then
    error_exit "$WORKDIR is not running. Do '$WORKDIR start'."
  fi

  if ! start_suibase_daemon_as_needed; then
    error_exit "proxy server is not running. Do '$WORKDIR start'."
  fi

  local _DISP
  if [ "$_DEBUG_OPTION" = "true" ]; then
    _DISP="debug"
  else
    if [ "$_JSON_PARAM" = "true" ]; then
      _DISP="data"
    else
      _DISP="display"
    fi
  fi

  # Output, if any, will be in JSON_RESP
  unset JSON_RESP
  get_suibase_daemon_status "$_DISP"
  if [ -z "$JSON_RESP" ]; then
    error_exit "proxy server not responding. Is it running? do '$WORKDIR status' to check."
  fi

  # We control the API and the "code":"value" and "message":"value" combination is sent
  # only when there is an error...
  update_JSON_VALUE "code" "$JSON_RESP"
  if [ -n "$JSON_VALUE" ]; then
    update_JSON_VALUE "message" "$JSON_RESP"
    if [ -n "$JSON_VALUE" ]; then
      echo "$JSON_RESP"
      error_exit "Unexpected proxy server RPC response."
    fi
  fi

  # We let the proxy server produce a pretty formating and just
  # blindly display. The proxy server conveniently provide both
  # a typical "JSON metrics" API and this optional alternative
  # display for when the client has weak JSON and string formating
  # capability... like bash.

  if [ "$_DISP" == "data" ]; then
    echo "$JSON_RESP"
  else
    update_JSON_VALUE "display" "$JSON_RESP"
    echo -e "$JSON_VALUE"
  fi

  if [ "$_DEBUG_OPTION" = true ]; then
    update_JSON_VALUE "debug" "$JSON_RESP"
    # Insert a few ****** between each InputPort
    JSON_VALUE=$(echo "$JSON_VALUE" | sed -e 's/Some(InputPort/\n*****************************\nSome(InputPort/g')
    # Insert a few ----- between each TargetServer
    JSON_VALUE=$(echo "$JSON_VALUE" | sed -e 's/Some(TargetServer/\n------------\nSome(TargetServer/g')
    # Insert "" before all_server_stats
    JSON_VALUE=$(echo "$JSON_VALUE" | sed -e 's/all_servers_stats/\n--\nall_servers_stats/g')
    echo -e "$JSON_VALUE"
  fi

  # Total Request Counts
  #   Success first attempt      10291212   100 %
  #   Success after retry                     0
  #   Failure after retry                     0
  #   Failure on network down                 0
  #   Bad Request                             0
  #
  # alias    Health %   Load  %   RespT ms    Retry %
  #            +100.1      98.1     102.12   >0.001
  #               0.0       2.2    >1 secs       -

  update_JSON_VALUE "proxyEnabled" "$JSON_RESP"
  if [ -n "$JSON_VALUE" ] && [ "$JSON_VALUE" != "true" ]; then
    echo "-----"
    warn_user "proxy server still initializing for $WORKDIR"
    echo "proxy server currently not monitoring $WORKDIR. This could"
    echo "be normal if it was disabled/(re)enabled just a moment ago."
  fi
}
export -f show_suibase_daemon_get_links
