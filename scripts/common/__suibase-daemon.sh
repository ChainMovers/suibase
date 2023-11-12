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
  # shellcheck disable=SC2153
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

  if [ ! "$SUIBASE_DAEMON_VERSION_INSTALLED" = "$SUIBASE_DAEMON_VERSION_SOURCE_CODE" ]; then
    true
    return
  fi

  if [ "${CFG_proxy_enabled:?}" = "dev" ]; then
    # fstat the binaries for differences (target/debug is normally not deleted while "dev").
    local _SRC="$SUIBASE_DAEMON_BUILD_DIR/target/debug/$SUIBASE_DAEMON_NAME"
    local _DST="$SUIBASE_DAEMON_BIN"
    if [ ! -f "$_SRC" ]; then
      true
      return
    fi
    if ! cmp --silent "$_SRC" "$_DST"; then
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

is_suibase_daemon_running() {
  # Quickly determine if the daemon is running, does not check if responding.
  # Has the side effect of updating the SUIBASE_DAEMON_PID variable.
  update_SUIBASE_DAEMON_PID_var

  if [ -n "$SUIBASE_DAEMON_PID" ]; then
    true
    return
  fi

  false
  return
}

start_suibase_daemon() {
  # success/failure is reflected by the SUIBASE_DAEMON_PID var.
  # noop if the process is already started.
  if is_suibase_daemon_running; then
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

  echo "Starting $SUIBASE_DAEMON_NAME"

  if [ "${CFG_proxy_enabled:?}" = "dev" ]; then
    # Run it in the foreground and just exit when done.
    "$HOME"/suibase/scripts/common/run-suibase-daemon.sh foreground
    exit
  fi

  # Try until can confirm the suibase-daemon is running healthy, or exit
  # if takes too much time.
  end=$((SECONDS + 30))
  ALIVE=false
  AT_LEAST_ONE_SECOND=false
  for _i in {1..3}; do
    # Try to start a script that keeps alive the suibase-daemon.
    #
    # Will not try if there is already another instance running.
    #
    # run-suibase-daemon.sh is design to be flock protected and be silent.
    # All errors will be visible through the suibase-daemon own logs or by observing
    # which PID owns the flock file. So all output of the script (if any) can
    # safely be ignored to /dev/null.
    nohup "$HOME/suibase/scripts/common/run-suibase-daemon.sh" >/dev/null 2>&1 &

    while [ $SECONDS -lt $end ]; do
      if is_suibase_daemon_running; then
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
  # TODO currently unused. Revisit if needed versus a self-exit design.

  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUIBASE_DAEMON_PID_var
  if [ -n "$SUIBASE_DAEMON_PID" ]; then
    echo "Stopping $SUIBASE_DAEMON_NAME (process pid $SUIBASE_DAEMON_PID)"

    # TODO This will just restart the daemon... need to actually kill the parents as well!
    kill -s SIGTERM "$SUIBASE_DAEMON_PID"

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

update_suibase_daemon_as_needed() {
  start_suibase_daemon_as_needed "force-update"
}
export -f update_suibase_daemon_as_needed

start_suibase_daemon_as_needed() {

  # When _UPGRADE_ONLY=true:
  #  - Always check to upgrade the daemon.
  #  - Do not start, but restart if *already* running.
  # else:
  #  - if 'proxy_enabled' is true, then check to
  #    upgrade and (re)starts.
  #
  local _UPGRADE_ONLY
  local _SUPPORT_PROXY
  if [ "$1" = "force-update" ]; then
    _UPGRADE_ONLY=true
    _SUPPORT_PROXY=true
  else
    # Verify from suibase.yaml if the suibase daemon should be started.
    _UPGRADE_ONLY=false
    if [ "${CFG_proxy_enabled:?}" = "false" ]; then
      _SUPPORT_PROXY=false
    else
      _SUPPORT_PROXY=true
    fi
  fi

  # Return 0 on success or not needed.

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
      if [ "$_UPGRADE_ONLY" = true ]; then
        # Restart only if already running.
        if [ -n "$SUIBASE_DAEMON_PID" ]; then
          restart_suibase_daemon
        fi
      else
        # (re)start
        if [ -z "$SUIBASE_DAEMON_PID" ]; then
          start_suibase_daemon
        else
          # Needed for the upgrade to take effect.
          restart_suibase_daemon
        fi
      fi
    else
      if [ -z "$SUIBASE_DAEMON_PID" ] && [ "$_UPGRADE_ONLY" = false ]; then
        # There was no upgrade, but the process need to be started.
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

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"getLinks\",\"params\":{\"workdir\":\"$WORKDIR_NAME\",\"$_DISP\":true}}"

  JSON_RESP=$(curl -x "" -s --location -X POST "http://${CFG_proxy_host_ip:?}:${CFG_suibase_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS")
}
export -f get_suibase_daemon_status

show_suibase_daemon_get_links() {
  local _DEBUG_OPTION=$1
  local _JSON_PARAM=$2

  local _USER_REQUEST
  _USER_REQUEST=$(get_key_value "$WORKDIR" "user_request")

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

    # Append help to the display.
    JSON_VALUE="$JSON_VALUE\n* is the load-balanced range on first attempt.\nOn retry, another OK server is selected in order shown in table."

    if [[ "${CFG_terminal_color:?}" == 'true' ]]; then
      # Color OK in blue in tables (have two spaces after)
      JSON_VALUE=$(echo "$JSON_VALUE" | sed -e '1,$s/OK  /\x1B[1;34mOK\x1B[0m  /g')
      # Color OK in green in top status
      JSON_VALUE=$(echo "$JSON_VALUE" | sed -e '1,$s/multi-link RPC: OK/multi-link RPC: \x1B[1;32mOK\x1B[0m/g')
      # Color DOWN in red.
      JSON_VALUE=$(echo "$JSON_VALUE" | sed -e '1,$s/DOWN/\x1B[1;31mDOWN\x1B[0m/g')
      # Color the load balancing bar.
      JSON_VALUE=$(echo "$JSON_VALUE" | sed -e '1,$s/*/\x1B[42;37m*\x1B[0m/g')
    fi
    echo -e "$JSON_VALUE"
  fi

  if [ "$_DEBUG_OPTION" = true ]; then
    update_JSON_VALUE "debug" "$JSON_RESP"
    # Insert a few ****** between each InputPort
    # shellcheck disable=SC2001
    JSON_VALUE=$(echo "$JSON_VALUE" | sed -e 's/Some(InputPort/\n*****************************\nSome(InputPort/g')
    # Insert a few ----- between each TargetServer
    # shellcheck disable=SC2001
    JSON_VALUE=$(echo "$JSON_VALUE" | sed -e 's/Some(TargetServer/\n------------\nSome(TargetServer/g')
    # Insert "" before all_server_stats
    # shellcheck disable=SC2001
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
    warn_user "proxy server still initializing for $WORKDIR_NAME"
    echo "proxy server currently not monitoring $WORKDIR_NAME. This could"
    echo "be normal if it was disabled/(re)enabled just a moment ago."
  fi
}
export -f show_suibase_daemon_get_links

notify_suibase_daemon_fs_change() {
  # Best-effort notification to the suibase-daemon that a state/config changed on
  # the filesystem for that workdir. Purposely leave the "path" parameter vague
  # so that the daemon can audit/check *everything*.

  if ! is_suibase_daemon_running; then
    return
  fi

  if [ -z "$WORKDIR_NAME" ]; then
    return
  fi

  local _HEADERS="Content-Type: application/json"

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"fsChange\",\"params\":{\"path\":\"$WORKDIR_NAME\"}}"

  curl --max-time 5 -x "" -s --location -X POST "http://${CFG_proxy_host_ip:?}:${CFG_suibase_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS" >/dev/null 2>&1 &
}
export -f notify_suibase_daemon_fs_change

# Step done after a network publication.
#
# Allows the suibase-daemon to learn about the packageid and do some
# more sanity checks.
#
do_suibase_daemon_post_publish() {
  local _TOML_PATH=$1
  local _NAME=$2
  local _UUID=$3
  local _TIMESTAMP=$4
  local _ID=$5

  if ! is_suibase_daemon_running; then
    # TODO attempt to restart daemon here before failing.
    error_exit "suibase-daemon not running. Do '$WORKDIR start' and try again."
  fi

  if [ -z "$WORKDIR_NAME" ]; then
    error_exit "do_suibase_daemon_post_publish internal error: Missing WORKDIR_NAME"
  fi

  if [ -z "$_NAME" ]; then
    error_exit "do_suibase_daemon_post_publish internal error: Missing NAME"
  fi

  if [ -z "$_UUID" ]; then
    error_exit "do_suibase_daemon_post_publish internal error: Missing UUID"
  fi

  if [ -z "$_ID" ]; then
    error_exit "do_suibase_daemon_post_publish internal error: Missing ID"
  fi

  local _HEADERS="Content-Type: application/json"

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"postPublish\",\"params\":{\"workdir\":\"$WORKDIR_NAME\", \"move_toml_path\": \"$_TOML_PATH\", \"package_name\": \"$_NAME\", \"package_uuid\": \"$_UUID\", \"package_timestamp\": \"$_TIMESTAMP\", \"package_id\": \"$_ID\"}}"

  _RESULT=$(curl --max-time 5 -x "" -s --location -X POST "http://${CFG_proxy_host_ip:?}:${CFG_suibase_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS")
  update_JSON_VALUE "result" "$_RESULT"
  if [ "$JSON_VALUE" != "true" ]; then
    echo "post-publish error: [$_RESULT] [$JSON_VALUE]"
    error_exit "A post publication operation failed. Recommended to try to publish again".
  fi
}
export -f do_suibase_daemon_post_publish

# Step to perform prior to publication to a network to create
# a unique UUID for the package.
#
# Will update global variables: PACKAGE_UUID and PACKAGE_TIMESTAMP

# PACKAGE_TIMESTAMP is Unix EPOCH in milliseconds (remove last 3 digits for seconds).
export PACKAGE_UUID=""
export PACKAGE_TIMESTAMP=""
do_suibase_daemon_pre_publish() {
  # Best-effort notification to the suibase-daemon that a new modules
  # was successfully published.
  local _TOML_PATH=$1
  local _NAME=$2

  if [ -z "$WORKDIR_NAME" ]; then
    error_exit "do_suibase_daemon_pre_publish internal error: Missing WORKDIR_NAME"
  fi

  if [ -z "$_NAME" ]; then
    error_exit "do_suibase_daemon_pre_publish internal error: Missing NAME"
  fi

  # The suibase-daemon is needed to be working to get the PACKAGE_UUID.
  if ! is_suibase_daemon_running; then
    # TODO attempt to restart daemon here before failing.
    error_exit "suibase-daemon not running. Do '$WORKDIR start' and try again."
  fi

  local _HEADERS="Content-Type: application/json"

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"prePublish\",\"params\":{\"workdir\":\"$WORKDIR_NAME\", \"move_toml_path\": \"$_TOML_PATH\", \"package_name\": \"$_NAME\"}}"

  _RESULT=$(curl --max-time 5 -x "" -s --location -X POST "http://${CFG_proxy_host_ip:?}:${CFG_suibase_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS")
  update_JSON_VALUE "result" "$_RESULT"
  if [ "$JSON_VALUE" != "true" ]; then
    error_exit "do_suibase_daemon_pre_publish failed: [$_RESULT] [$JSON_VALUE]"
  fi

  # TODO Extract the package_uuid from the result.
  update_JSON_VALUE "info" "$_RESULT"
  if [ -z "$JSON_VALUE" ]; then
    error_exit "do_suibase_daemon_pre_publish failed (missing info): [$_RESULT]"
  fi

  local _INFO="$JSON_VALUE"
  # Extract from _INFO string the PACKAGE_UUID and PACKAGE_TIMESTAMP
  # The fields are comma seperated in the string.
  IFS=',' read -ra INFO_ARRAY <<<"$_INFO"

  # Assign array elements to variables
  PACKAGE_UUID=${INFO_ARRAY[0]}
  PACKAGE_TIMESTAMP=${INFO_ARRAY[1]}
}
export -f do_suibase_daemon_pre_publish
