# shellcheck shell=bash

# You must source __globals.sh and __apps.sh before __suibase-daemon.sh

# shellcheck disable=SC2034
declare -A sb_daemon_obj

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

  # Note: lock function is re-entrant. Won't block if this script is already holding the lock.
  cli_mutex_lock "suibase_daemon"

  # Check again while holding the lock.
  if is_suibase_daemon_running; then
    return
  fi

  # shellcheck disable=SC2153
  if [ ! -f "$SUIBASE_DAEMON_BIN" ]; then
     setup_error "$SUIBASE_DAEMON_NAME binary not found (build failed?)"
  fi

  echo "Starting $SUIBASE_DAEMON_NAME"
  mkdir -p "$SUIBASE_LOGS_DIR"

  # Try until can confirm the suibase-daemon is running healthy, or exit
  # if takes too much time.
  end=$((SECONDS + 50))
  ALIVE=false
  AT_LEAST_ONE_SECOND=false
  for _i in {1..5}; do
    # Try to start a script that keeps alive the suibase-daemon.
    #
    # Will not try if there is already another instance running.
    #
    # run-daemon.sh is design to be flock protected and be silent.
    # All errors will be visible through the suibase-daemon own logs or by observing
    # which PID owns the flock file. So all output of the script (if any) can
    # safely be ignored to /dev/null.
    #
    # "cli-call" param prevent run-daemon to try locking the CLI mutex.
    nohup "$SUIBASE_DIR/scripts/common/run-daemon.sh" suibase cli-call >"$SUIBASE_LOGS_DIR/run-daemon.log" 2>&1 &

    local _NEXT_RETRY=$((SECONDS + 10))
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
      if [ $SECONDS -gt $_NEXT_RETRY ]; then
        break
      fi
    done

    # If it is alive or timeout, then break the retry loop.
    if [ "$ALIVE" = true ] || [ $SECONDS -ge $end ]; then
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
      setup_error "$SUIBASE_DAEMON_NAME binary not found (build failed?)"
    else
      setup_error "$SUIBASE_DAEMON_NAME not responding. Try again? (may be the host is too slow?)."
    fi
  fi

  echo "$SUIBASE_DAEMON_NAME started (process pid $SUIBASE_DAEMON_PID)"
}
export -f start_suibase_daemon

wait_for_json_rpc_up() {
  local _CMD=$1 # a specific workdir, "any", "none" or "exclude-localnet"

  # Array of valid workdirs
  local _WORKDIRS_VALID_LIST=("localnet" "testnet" "devnet" "mainnet")
  # Verify if $_CMD is one of the element of _WORKDIRS_VALID_LIST
  local _WORKDIR_VALID=false
  for _WORKDIR in "${_WORKDIRS_VALID_LIST[@]}"; do
    if [ "$_WORKDIR" = "$_CMD" ]; then
      _WORKDIR_VALID=true
      break
    fi
  done

  if [ "$_WORKDIR_VALID" = false ] && [ "$_CMD" != "any" ] && [ "$_CMD" != "none" ] && [ "$_CMD" != "exclude-localnet" ]; then
    warn_user "wait_for_json_rpc_up unexpected workdir name: $_CMD"
    return
  fi

  # Block for up to 20 seconds for the JSON-RPC to be confirm up.
  #
  # The first parameter controls which workdir can be checked.
  #
  local _WORKDIRS_ORDERED_LIST
  if $_WORKDIR_VALID; then
    _WORKDIRS_ORDERED_LIST=("$_CMD")
  elif [ "$_CMD" == "any" ] || [ "$_CMD" == "none" ]; then
     # Try one of the remote workdirs (localnet might not be started yet).
    _WORKDIRS_ORDERED_LIST=("testnet" "devnet" "mainnet")
  else
    _WORKDIRS_ORDERED_LIST=("$WORKDIR_NAME")
    # Append the other workdirs in the order of the _WORKDIRS_VALID_LIST
    for _WORKDIR in "${_WORKDIRS_VALID_LIST[@]}"; do
      if [ "$_WORKDIR" != "$WORKDIR_NAME" ]; then
        _WORKDIRS_ORDERED_LIST+=("$_WORKDIR")
      fi
    done
  fi
  local _WORKDIRS_STARTED_LIST=()

  # Remove workdirs that are exluded or not started by the user.
  local _WORKDIR_NAME
  local _WORKDIR_FOUND=false
  for _WORKDIR_NAME in "${_WORKDIRS_ORDERED_LIST[@]}"; do
    if [ "$_WORKDIR_NAME" = "localnet" ] && [ "$_CMD" = "exclude-localnet" ]; then
      continue;
    fi
    local _USER_REQUEST
    _USER_REQUEST=$(get_key_value "$_WORKDIR_NAME" "user_request")
    if [ "$_USER_REQUEST" = "start" ]; then
      _WORKDIR_FOUND=true
      # Add _WORKDIR_NAME to _WORKDIRS_STARTED_LIST
      _WORKDIRS_STARTED_LIST+=("$_WORKDIR_NAME")
    fi
  done

  # If no workdir is found to be started by user, then just return.
  if [ "$_WORKDIR_FOUND" = false ]; then
    return
  fi

  local _SUI_RESP
  local _AT_LEAST_ONE_DOT=false
  local _JSON_RPC_UP=false
  local _WORKDIRS_IDX=0
  for _i in {1..20}; do

    # Cycle through the _WORKDIRS_STARTED_LIST
    _WORKDIR_NAME=${_WORKDIRS_STARTED_LIST[$_WORKDIRS_IDX]}
    _WORKDIRS_IDX=$(((_WORKDIRS_IDX + 1) % ${#_WORKDIRS_STARTED_LIST[@]}))

    # shellcheck disable=SC2153
    local _SUI_EXEC="$WORKDIRS/$_WORKDIR_NAME/sui-exec"

    # Do a sui "client gas" call, and verify that
    # it returns something valid.

    _SUI_RESP=$("$_SUI_EXEC" client gas --json 2>&1)
    # echo "client gas for $_WORKDIR_NAME returns [$_SUI_RESP]"
    if [ -n "$_SUI_RESP" ]; then
      # Valid if starts with [ and end with ] ... or respond with "no addresses".
      if [[ "$_SUI_RESP" =~ ^\[.*\]$ ]] || [[ "$_SUI_RESP" =~ "No managed addresses" ]]; then
        _JSON_RPC_UP=true
        break
      fi
    fi

    if [ "$_i" -eq 1 ] || [ "$_i" -eq 4 ] || [ "$_i" -eq 12 ]; then
      # *Kick* the daemons in case they have not detected yet the "start" state.
      trig_daemons_refresh
    fi

    # Wait one second before trying again.
    if [ "$_i" -eq 5 ]; then
      echo -n "Verifying JSON-RPC is responding..."
      _AT_LEAST_ONE_DOT=true
    fi
    if [ "$_i" -gt 5 ]; then
      echo -n "."
    fi
    sleep 1
  done
  if [ "$_AT_LEAST_ONE_DOT" = true ]; then
    echo
    if [ "$_JSON_RPC_UP" = true ]; then
      echo "JSON-RPC is up"
    else
      echo "JSON-RPC not responding. Try again later?"
    fi
  fi
}
export -f wait_for_json_rpc_up

restart_suibase_daemon() {
  # Note: lock function is re-entrant. Won't block if this script is already holding the lock.
  cli_mutex_lock "suibase_daemon"

  update_SUIBASE_DAEMON_PID_var
  if [ -z "$SUIBASE_DAEMON_PID" ]; then
    start_suibase_daemon
  else
    # This is a clean restart of the daemon (Ctrl-C). Should be fast.
    local _OLD_PID="$SUIBASE_DAEMON_PID"
    stop_suibase_daemon
    echo -n "Restarting $SUIBASE_DAEMON_NAME"
    end=$((SECONDS + 30))
    while [ $SECONDS -lt $end ]; do
      sleep 1
      update_SUIBASE_DAEMON_PID_var
      if [ -n "$SUIBASE_DAEMON_PID" ] && [ "$SUIBASE_DAEMON_PID" != "$_OLD_PID" ]; then
        echo " (new pid $SUIBASE_DAEMON_PID)"
        return
      fi
    done
    echo " (failed)"
    echo "Try again with '~/suibase/restart'"
  fi
}
export -f restart_suibase_daemon

stop_suibase_daemon() {
  # Note: lock function is re-entrant. Won't block if this script is already holding the lock.
  cli_mutex_lock "suibase_daemon"

  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUIBASE_DAEMON_PID_var
  if [ -n "$SUIBASE_DAEMON_PID" ]; then
    local _OLD_PID="$SUIBASE_DAEMON_PID"
    echo "Stopping $SUIBASE_DAEMON_NAME (pid $_OLD_PID)"

    # This exit the daemon cleanly... consider that run-daemon.sh may restart it immediately.
    kill -s SIGTERM "$SUIBASE_DAEMON_PID"

    # Make sure it was killed.
    end=$((SECONDS + 15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_SUIBASE_DAEMON_PID_var
      if [ -z "$SUIBASE_DAEMON__PID" ] || [ "$SUIBASE_DAEMON_PID" != "$_OLD_PID" ]; then
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

export SUIBASE_DAEMON_PID=""
update_SUIBASE_DAEMON_PID_var() {

  local _PID
  _PID=$(lsof -t -a -c suibase "$SUIBASE_TMP_DIR"/"$SUIBASE_DAEMON_NAME".lock 2>/dev/null)
  local _LSOF_RETURN_CODE=$?

  #shellcheck disable=SC2181
  if [ $_LSOF_RETURN_CODE -eq 0 ]; then
    # Trim potential whitespace and carriage return from _PID
    _PID=$(echo "$_PID" | tr -d '[:space:]')

    # Verify _PID is a number. Otherwise, assume it is an error.
    if [[ "$_PID" =~ ^[0-9]+$ ]]; then
      export SUIBASE_DAEMON_PID="$_PID"
      return
    fi
  fi

  # For error case or daemon not running...

  # Check if lsof is not installed, then inform the user to install it.
  if ! is_installed lsof; then
    setup_error "The CLI command 'lsof' must be installed to run Suibase scripts"
  fi

  # If the lock file exists but apparently not working, use ps as a fallback
  # (in rare scenario where shell have permission issues).
  local _PID
  _PID=$(get_process_pid "$SUIBASE_DAEMON_BIN")
  if [ -n "$_PID" ] && [ "$_PID" != "NULL" ]; then
    SUIBASE_DAEMON_PID="$_PID"
    return
  fi

  SUIBASE_DAEMON_PID=""
}
export -f update_SUIBASE_DAEMON_PID_var

export SUIBASE_DAEMON_STARTED=false
start_suibase_daemon_as_needed() {

  # Return 0 on success or not needed.

  # Changes to true if the daemon is tentatively started
  # anywhere within this call.
  SUIBASE_DAEMON_STARTED=false

  init_app_obj sb_daemon_obj "suibase_daemon" ""
  vcall sb_daemon_obj "set_local_vars"
  # vcall sb_daemon_obj "print"

  if [ "${sb_daemon_obj["is_installed"]}" != "true" ]; then

    local _PERFORM_INSTALL=false

    # Check SUIBASE_DAEMON_UPGRADING to prevent multiple attempts to upgrade
    # within the same CLI call.

    if [ "$SUIBASE_DAEMON_UPGRADING" = "false" ]; then
      _PERFORM_INSTALL=true

      # To try to minimize race conditions, wait until a concurrent script
      # is done doing the same. Not perfect... good enough.
      #
      # This also hint the VSCode extension to step back from using the
      # backend while this is on-going.
      local _CHECK_IF_NEEDED_AGAIN=false
      local _FIRST_ITERATION=true
      while [ -f /tmp/.suibase/suibase-daemon-upgrading ]; do
        if $_FIRST_ITERATION; then
          echo "Waiting for concurrent script to finish upgrading suibase daemon..."
          _FIRST_ITERATION=false
        fi
        sleep 1
        _PERFORM_INSTALL=false
      done

      # Note: lock function is re-entrant. Won't block if this script is already holding the lock.
      cli_mutex_lock "suibase_daemon"

      if [ "$_PERFORM_INSTALL" = false ]; then
        # Was block by another script... check again if the upgrade is still needed.
        vcall sb_daemon_obj "set_local_vars"

        if [ "${sb_daemon_obj["is_installed"]}" != "true" ]; then
          _PERFORM_INSTALL=true
        fi
      fi
    fi

    if [ "$_PERFORM_INSTALL" = true ]; then
      progress_suibase_daemon_upgrading
      local _OLD_VERSION="${sb_daemon_obj["local_bin_version"]}"
      # OLD build_suibase_daemon
      # OLD update_SUIBASE_DAEMON_VERSION_INSTALLED
      cli_mutex_lock "suibase_daemon"
      vcall sb_daemon_obj "install"
      vcall sb_daemon_obj "set_local_vars"
      local _NEW_VERSION="${sb_daemon_obj["local_bin_version"]}"
      if [ "${sb_daemon_obj["is_installed"]}" != "true" ] || [ -z "$_NEW_VERSION" ]; then
        setup_error "Failed to install ${sb_daemon_obj["assets_name"]}"
      fi

      local _WAS_UPGRADED=false
      if [ "$_OLD_VERSION" != "$_NEW_VERSION" ]; then
        if [ -n "$_OLD_VERSION" ]; then
          echo "${sb_daemon_obj["assets_name"]} upgraded from $_OLD_VERSION to $_NEW_VERSION"
          _WAS_UPGRADED=true
        else
          echo "${sb_daemon_obj["assets_name"]} $_NEW_VERSION installed"
        fi
      fi

      # Check if restart needed when binary was upgraded.
      update_SUIBASE_DAEMON_PID_var
      if [ -n "$SUIBASE_DAEMON_PID" ] && [ "$_WAS_UPGRADED" = true ]; then
        restart_suibase_daemon
        SUIBASE_DAEMON_STARTED=true
      fi
    fi
  fi

  if [ ! "$SUIBASE_DAEMON_STARTED" = "true" ]; then
    update_SUIBASE_DAEMON_PID_var
    if [ -z "$SUIBASE_DAEMON_PID" ]; then
      # There was no upgrade, but the process need to be started.
      start_suibase_daemon
      SUIBASE_DAEMON_STARTED=true
    fi
  fi

  return 0
}
export -f start_suibase_daemon_as_needed

# The response is written in global JSON_RESP
get_suibase_daemon_status() {
  local _DISP=$1 # one of "data", "debug" or "display"

  local _HEADERS="Content-Type: application/json"

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"getLinks\",\"params\":{\"workdir\":\"$WORKDIR_NAME\",\"$_DISP\":true}}"

  export JSON_RESP
  JSON_RESP=$(curl --max-time 2 -x "" -s --location -X POST "http://${CFG_proxy_host_ip:?}:${CFG_suibase_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS")
}
export -f get_suibase_daemon_status

show_suibase_daemon_get_links() {
  local _DEBUG_OPTION=$1
  local _JSON_PARAM=$2

  local _USER_REQUEST
  # shellcheck disable=SC2153
  _USER_REQUEST=$(get_key_value "$WORKDIR" "user_request")

  if [ "$_USER_REQUEST" = "stop" ]; then
    error_exit "$WORKDIR is not running. Do '$WORKDIR start'."
  fi

  if ! start_suibase_daemon_as_needed; then
    error_exit "proxy server is not running. Do '$WORKDIR start'."
  fi

  if [ $SUIBASE_DAEMON_STARTED = true ]; then
    # Give a moment for the daemon to start.
    wait_for_json_rpc_up "${WORKDIR_NAME}"
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

  curl --max-time 1 -x "" -s --location -X POST "http://${CFG_proxy_host_ip:?}:${CFG_suibase_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS" >/dev/null 2>&1 &
}
export -f notify_suibase_daemon_fs_change

notify_suibase_daemon_workdir_change() {
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

  local _JSON_PARAMS="{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"workdirRefresh\",\"params\":{\"workdir\":\"$WORKDIR_NAME\"}}"

  curl --max-time 1 -x "" -s --location -X POST "http://${CFG_proxy_host_ip:?}:${CFG_suibase_api_port_number:?}" -H "$_HEADERS" -d "$_JSON_PARAMS" >/dev/null 2>&1 &
}
export -f notify_suibase_daemon_workdir_change

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
