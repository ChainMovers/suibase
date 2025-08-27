# shellcheck shell=bash

# You must source __globals.sh before __walrus-relay-process.sh

# One file provides the status data:
#   ~/suibase/workdirs/{testnet,mainnet}/walrus-relay/status.yaml
#
# The status.yaml format is:
#   status: "DISABLED", "INITIALIZING", "OK", "DOWN"
#   last_error: "error message"
#   local_connectivity: "OK", "ERROR", "UNKNOWN"
#   proxy_port: 45852
#   local_port: 45802
#

# Supported functions:
#      - walrus_relay_status displays info with the following priority (overriding status.yaml):
#          DISABLED - when suibase.yaml walrus_relay_enabled is not true
#          STOPPED - when user explicitly requested to stop the services
#          NOT RUNNING - when suibase-daemon is not running
#          INITIALIZING - when daemon is running but status.yaml doesn't exist yet
#          Otherwise uses status from status.yaml (DISABLED, INITIALIZING, OK, DOWN)
#
#      - walrus_relay_enable
#          Modify the ~/suibase/workdirs/{testnet,mainnet}/suibase.yaml file by
#          adding or updating 'walrus_relay_enabled: true'.
#
#      - walrus_relay_disable
#          Modify the ~/suibase/workdirs/{testnet,mainnet}/suibase.yaml file by
#          adding or updating 'walrus_relay_enabled: false'.

WALRUS_RELAY_DIR="$WORKDIRS/$WORKDIR/walrus-relay"
WALRUS_RELAY_STATUS_FILE="$WALRUS_RELAY_DIR/status.yaml"
WALRUS_RELAY_SUIBASE_YAML="$WORKDIRS/$WORKDIR/suibase.yaml"

update_walrus_relay_status_yaml() {
  # Parse the status.yaml file
  if [ -f "$WALRUS_RELAY_STATUS_FILE" ]; then
    eval "$(parse_yaml "$WALRUS_RELAY_STATUS_FILE" "WRELAY_")"
    # Note: WRELAY_proxy_port should come from status.yaml to detect sync issues
    # If missing, daemon needs to be updated to include port info
  else
    # Use default values if file doesn't exist yet...
    WRELAY_status="INITIALIZING"
    WRELAY_last_error=""
    WRELAY_local_connectivity="UNKNOWN"
    # Load config values from CFG variables
    WRELAY_proxy_port="${CFG_walrus_relay_proxy_port:?}"
    WRELAY_local_port="${CFG_walrus_relay_local_port:?}"
  fi
}

# Lazy loading helper - only loads status.yaml when first accessed
ensure_walrus_relay_status_loaded() {
  if [ -z "${WRELAY_status:-}" ]; then
    update_walrus_relay_status_yaml
  fi
}

walrus_relay_echo_status_color() {
  # Echo the status in color within fix spaces.
  local _status="$1"

  # No color:
  #  DISABLED
  #
  # Blue:
  #  OK
  #
  # Yellow:
  #  INITIALIZING
  #
  # Red
  #  DOWN
  #  NOT RUNNING
  case "$_status" in
  "INITIALIZING" | "OK")
    echo_blue "$_status"
    ;;
  "DOWN" | "NOT RUNNING")
    echo_red "$_status"
    ;;
  *)
    echo -n "$_status"
    ;;
  esac
}
export -f walrus_relay_echo_status_color

walrus_relay_echo_status_line() {
  # Display walrus relay status line with consistent formatting
  # Parameters: label, support_enabled, backend_pid, status, info
  local _label="$1"
  local _support_enabled="$2"
  local _local_backend_pid="$3"
  local _status="$4"
  local _info="$5"

  printf "%-17s: " "$_label"

  if [ "$_support_enabled" = false ]; then
    walrus_relay_echo_status_color "$_status"
    echo
  elif [ -n "$_local_backend_pid" ]; then
    # Show status with PID
    walrus_relay_echo_status_color "$_status"
    echo -n " ( pid "
    echo_blue "$_local_backend_pid"
    echo -n " ) $_info"
    echo
  else
    # Show status without PID
    walrus_relay_echo_status_color "$_status"
    if [ -n "$_info" ]; then
      echo -n " $_info"
    fi
    echo
  fi
}
export -f walrus_relay_echo_status_line

export WALRUS_RELAY_STATUS="DOWN"
export WALRUS_RELAY_INFO=""

walrus_relay_status() {
  # Can be either "verbose" or "quiet"
  #
  # When "quiet" the WALRUS_RELAY_STATUS and WALRUS_RELAY_INFO variables are still updated
  # but there is no stdout output.
  #
  # When "verbose" a status line is printed to stdout. Examples:
  #  Walrus Relay     : DISABLED
  #  Walrus Relay     : OK ( pid 1223131 ) http://localhost:45852
  #  Walrus Relay     : INITIALIZING ( pid 1223131 ) http://localhost:45852
  #  Walrus Relay     : NOT RUNNING http://localhost:45852
  #  Walrus Relay     : DOWN http://localhost:45852
  #

  local verbosity=$1
  local suibase_daemon_pid=$2
  local user_request=$3

  if ! is_walrus_supported_by_workdir; then
    WALRUS_RELAY_STATUS="DISABLED"
    WALRUS_RELAY_INFO=""
    return
  fi

  # Get the info from the status.yaml file and process PID
  ensure_walrus_relay_status_loaded
  local _proxy_port="$WRELAY_proxy_port"

  # Get backend process PID (conditionally source if not already loaded)
  update_WALRUS_RELAY_PROCESS_PID_var

  # Determine support and process status
  local _support_enabled=true # Will change to false if disabled in suibase.yaml
  local _local_backend_pid=""
  local _status_for_display=""
  local _info_for_display=""

  if [ "${CFG_walrus_relay_enabled:-false}" != "true" ]; then
    _support_enabled=false
    _status_for_display="DISABLED"
  elif [ "$user_request" = "stop" ]; then
    _status_for_display="STOPPED"
  elif [ -z "$suibase_daemon_pid" ]; then
    # Suibase daemon not running - walrus relay can't work
    _local_backend_pid=""  # No PID because daemon is down
    _status_for_display="NOT RUNNING"
    # Get expected port from config
    local _config_port="${CFG_walrus_relay_proxy_port:?}"
    _info_for_display="http://localhost:$_config_port"
  else
    # Daemon is running and walrus relay is enabled
    _local_backend_pid="$WALRUS_RELAY_PROCESS_PID"  # May be empty if not started yet

    # Check if status.yaml exists - if not, daemon doesn't support walrus relay yet
    if [ ! -f "$WALRUS_RELAY_STATUS_FILE" ]; then
      _status_for_display="INITIALIZING"
      _info_for_display=""  # No URL when initializing
    else
      # Check for transient state: config enabled but status.yaml shows DISABLED
      # This happens when daemon hasn't caught up to config change yet
      if [ "${WRELAY_status:-}" = "DISABLED" ]; then
        _status_for_display="INITIALIZING"
        _info_for_display=""  # No URL when initializing
      else
        _status_for_display="${WRELAY_status:-DOWN}"
      fi
      # Only show URL when actually working
      if [ "$_status_for_display" = "OK" ]; then
        # Get expected port from config
        local _config_port="${CFG_walrus_relay_proxy_port:?}"

        # Check for port synchronization issue
        if [ -n "${WRELAY_proxy_port:-}" ] && [ "$WRELAY_proxy_port" != "$_config_port" ]; then
          _info_for_display="http://localhost:? (transitioning from: $WRELAY_proxy_port to $_config_port)"
        elif [ -n "${WRELAY_proxy_port:-}" ]; then
          _info_for_display="http://localhost:$WRELAY_proxy_port"
        else
          # Daemon doesn't provide port info yet, use config port
          _info_for_display="http://localhost:$_config_port"
        fi
      else
        _info_for_display=""
      fi
    fi
  fi

  # Set the export variables for use by echo_process and other callers
  WALRUS_RELAY_SUPPORT_ENABLED=$_support_enabled
  WALRUS_RELAY_BACKEND_PID=$_local_backend_pid
  WALRUS_RELAY_STATUS=$_status_for_display
  WALRUS_RELAY_INFO=$_info_for_display

  if [ "$verbosity" = "verbose" ]; then
    walrus_relay_echo_status_line "Walrus Relay" "$WALRUS_RELAY_SUPPORT_ENABLED" "$WALRUS_RELAY_BACKEND_PID" "$WALRUS_RELAY_STATUS" "$WALRUS_RELAY_INFO"
    
    # Show stats if daemon is running and stats are available
    if [ -n "$suibase_daemon_pid" ] && [ "$WALRUS_RELAY_SUPPORT_ENABLED" = true ]; then
      # Get stats from API in quiet mode and display them
      local _stats_response
      _stats_response=$(curl -s -X POST -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getWalrusRelayStats\",\"params\":{\"workdir\":\"$WORKDIR\",\"summary\":true},\"id\":1}" \
        "http://localhost:${CFG_daemon_port:-44399}" 2>/dev/null)
      
      if [ -n "$_stats_response" ] && ! echo "$_stats_response" | jq -e '.error' >/dev/null 2>&1; then
        local _total_requests _successful_requests _failed_requests
        _total_requests=$(echo "$_stats_response" | jq -r '.result.summary.totalRequests // 0')
        _successful_requests=$(echo "$_stats_response" | jq -r '.result.summary.successfulRequests // 0')
        _failed_requests=$(echo "$_stats_response" | jq -r '.result.summary.failedRequests // 0')
        
        if [ "$_total_requests" -gt 0 ] || [ "$_successful_requests" -gt 0 ] || [ "$_failed_requests" -gt 0 ]; then
          echo "                 : Stats: ${_total_requests} total, ${_successful_requests} success, ${_failed_requests} failed"
        fi
      fi
    fi
    
    # Show additional help/error info when needed
    if [ "$WALRUS_RELAY_SUPPORT_ENABLED" = true ] && [ -z "$suibase_daemon_pid" ]; then
      echo
      echo "To run services do '$WORKDIR start'"
      echo
    elif [ "$WALRUS_RELAY_SUPPORT_ENABLED" = false ] && [ "$WALRUS_RELAY_STATUS" = "DISABLED" ]; then
      echo "To enable do '$WORKDIR wal-relay enable'"
    fi

    if [ -n "${WRELAY_last_error:-}" ]; then
      echo_red "Error: "
      echo "${WRELAY_last_error}"
    fi
  fi
}
export -f walrus_relay_status

walrus_relay_enable() {
  local verbosity=$1

  if ! is_walrus_supported_by_workdir; then
    setup_error "Walrus relay is only supported for testnet and mainnet"
  fi

  # Check if suibase.yaml exists
  if [ ! -f "$WALRUS_RELAY_SUIBASE_YAML" ]; then
    setup_error "Cannot find suibase.yaml at [$WALRUS_RELAY_SUIBASE_YAML]"
  fi

  # Check if walrus relay is already enabled
  local already_enabled=false
  if [ "${CFG_walrus_relay_enabled:-false}" = "true" ]; then
    already_enabled=true
  else
    # Update suibase.yaml
    if grep -q "^walrus_relay_enabled:" "$WALRUS_RELAY_SUIBASE_YAML"; then
      # Replace existing line
      sed -i.bak "s/^walrus_relay_enabled:.*/walrus_relay_enabled: true/" "$WALRUS_RELAY_SUIBASE_YAML" && rm "$WALRUS_RELAY_SUIBASE_YAML.bak"
    else
      # Add new line
      echo "walrus_relay_enabled: true" >> "$WALRUS_RELAY_SUIBASE_YAML"
    fi

    # Check if suibase daemon is running and start if needed
    exit_if_sui_binary_not_ok
    if ! start_suibase_daemon_as_needed; then
      echo "suibase services are not running. Do '$WORKDIR start'."
    fi
  fi

  # Display appropriate message
  if [ "$already_enabled" = true ]; then
    echo "Walrus relay already enabled"
  else
    echo "Walrus relay is now enabled"
  fi
}
export -f walrus_relay_enable

walrus_relay_disable() {
  local verbosity=$1

  if ! is_walrus_supported_by_workdir; then
    setup_error "Walrus relay is only supported for testnet and mainnet"
  fi

  # Check if suibase.yaml exists
  if [ ! -f "$WALRUS_RELAY_SUIBASE_YAML" ]; then
    setup_error "Cannot find suibase.yaml at $WALRUS_RELAY_SUIBASE_YAML"
  fi

  local already_disabled=false
  if [ "${CFG_walrus_relay_enabled:-false}" != "true" ]; then
    already_disabled=true
  else
    # Update suibase.yaml only if not already disabled
    if grep -q "^walrus_relay_enabled:" "$WALRUS_RELAY_SUIBASE_YAML"; then
      # Replace existing line
      sed -i.bak "s/^walrus_relay_enabled:.*/walrus_relay_enabled: false/" "$WALRUS_RELAY_SUIBASE_YAML" && rm "$WALRUS_RELAY_SUIBASE_YAML.bak"
    else
      # Add new line
      echo "walrus_relay_enabled: false" >> "$WALRUS_RELAY_SUIBASE_YAML"
    fi
  fi

  if [ "$verbosity" = "verbose" ]; then
    if [ "$already_disabled" = true ]; then
      echo "Walrus relay already disabled"
    else
      echo "Walrus relay is now disabled"
    fi
  fi
}
export -f walrus_relay_disable

repair_walrus_relay_symlink() {
  # Create/maintain permanent workdir-specific symlinks for process disambiguation
  # This allows reliable process detection between testnet/mainnet instances
  local _WORKDIR="$1"  # Required workdir parameter

  local _WORKDIR_PREFIX
  case "$_WORKDIR" in
    "testnet") _WORKDIR_PREFIX="t" ;;
    "mainnet") _WORKDIR_PREFIX="m" ;;
    *) _WORKDIR_PREFIX="${_WORKDIR:0:1}" ;;  # fallback: first char
  esac

  local _SOURCE_BINARY="$WORKDIRS/$_WORKDIR/bin/walrus-upload-relay"
  local _SYMLINK_BINARY="$WORKDIRS/$_WORKDIR/bin/${_WORKDIR_PREFIX}walrus-upload-relay"

  # Only create symlink if source binary exists
  if [ -f "$_SOURCE_BINARY" ]; then
    # Remove existing symlink if it's broken or points to wrong target
    if [ -L "$_SYMLINK_BINARY" ] && [ ! -e "$_SYMLINK_BINARY" ]; then
      rm -f "$_SYMLINK_BINARY"
    elif [ -L "$_SYMLINK_BINARY" ]; then
      local _CURRENT_TARGET
      _CURRENT_TARGET=$(readlink "$_SYMLINK_BINARY")
      if [ "$_CURRENT_TARGET" != "walrus-upload-relay" ]; then
        rm -f "$_SYMLINK_BINARY"
      fi
    fi

    # Create symlink if it doesn't exist
    if [ ! -e "$_SYMLINK_BINARY" ]; then
      ln -s "walrus-upload-relay" "$_SYMLINK_BINARY"
    fi
  fi
}
export -f repair_walrus_relay_symlink

start_walrus_relay_process() {
  # success/failure is reflected by the WALRUS_RELAY_PROCESS_PID var.
  # noop if the process is already started.

  if [ "${CFG_walrus_relay_enabled:?}" != "true" ]; then
    return
  fi

  exit_if_sui_binary_not_ok

  update_WALRUS_RELAY_PROCESS_PID_var
  if [ -n "$WALRUS_RELAY_PROCESS_PID" ]; then
    return
  fi

  echo "Starting $WORKDIR walrus-upload-relay"

  # Ensure required directories exist
  mkdir -p "$CONFIG_DATA_DIR_DEFAULT"
  mkdir -p "$WORKDIRS/$WORKDIR/walrus-relay"

  # Get configuration values
  local _RELAY_PORT="${CFG_walrus_relay_local_port:?}"
  local _WALRUS_CONFIG="$CONFIG_DATA_DIR_DEFAULT/walrus-config.yaml"
  local _RELAY_CONFIG="$CONFIG_DATA_DIR_DEFAULT/relay-config.yaml"

  # Get the workdir-specific symlinked binary name
  local _WORKDIR_PREFIX
  case "$WORKDIR" in
    "testnet") _WORKDIR_PREFIX="t" ;;
    "mainnet") _WORKDIR_PREFIX="m" ;;
    *) _WORKDIR_PREFIX="${WORKDIR:0:1}" ;;  # fallback: first char
  esac
  local _WORKDIR_BINARY="$WORKDIRS/$WORKDIR/bin/${_WORKDIR_PREFIX}walrus-upload-relay"

  # Ensure the symlink exists (repair function should have created it)
  repair_walrus_relay_symlink "$WORKDIR"

  # Verify symlinked binary exists
  if [ ! -f "$_WORKDIR_BINARY" ]; then
    setup_error "Workdir-specific walrus-upload-relay symlink not found at $_WORKDIR_BINARY"
  fi

  # Verify configuration files exist
  if [ ! -f "$_WALRUS_CONFIG" ]; then
    setup_error "walrus-config.yaml not found at $_WALRUS_CONFIG"
  fi

  # Create relay-config.yaml if it doesn't exist
  if [ ! -f "$_RELAY_CONFIG" ]; then
    echo "Creating default relay configuration at $_RELAY_CONFIG"
    cat > "$_RELAY_CONFIG" << EOF
tip_config: !no_tip
tx_freshness_threshold_secs: 36000
tx_max_future_threshold:
  secs: 30
  nanos: 0
EOF
  fi

  # Try up to 3 times to start the process.
  end=$((SECONDS + 30))
  ALIVE=false
  AT_LEAST_ONE_SECOND=false
  for _i in {1..3}; do
    rm -f "$WALRUS_RELAY_DIR/walrus-relay-process.log" >/dev/null 2>&1

    # Start walrus-upload-relay process using workdir-specific binary
    "$_WORKDIR_BINARY" \
      --context "$WORKDIR" \
      --walrus-config "$_WALRUS_CONFIG" \
      --server-address "0.0.0.0:$_RELAY_PORT" \
      --relay-config "$_RELAY_CONFIG" \
      >"$WALRUS_RELAY_DIR/walrus-relay-process.log" 2>&1 &

    # Loop until confirms can connect, or exit if takes too much time.
    while [ $SECONDS -lt $end ]; do
      # Health check via tip-config endpoint
      CHECK_ALIVE=$(curl -s "http://localhost:$_RELAY_PORT/v1/tip-config" 2>/dev/null)
      if [ -n "$CHECK_ALIVE" ]; then
        ALIVE=true
        break
      else
        echo -n "."
        sleep 1
        AT_LEAST_ONE_SECOND=true
      fi

      # Detect if should do a retry at starting it
      if [ -f "$WALRUS_RELAY_DIR/walrus-relay-process.log" ] && \
         grep -q "Address already in use\|failed to load\|panicked" "$WALRUS_RELAY_DIR/walrus-relay-process.log"; then
        # Sleep 2 seconds before retrying.
        for _j in {1..2}; do
          echo -n "."
          sleep 1
          AT_LEAST_ONE_SECOND=true
        done
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
    echo "walrus-upload-relay process not responding. Log contents:"
    if [ -f "$WALRUS_RELAY_DIR/walrus-relay-process.log" ]; then
      tail -10 "$WALRUS_RELAY_DIR/walrus-relay-process.log"
    fi
    setup_error "Failed to start walrus-upload-relay after 3 attempts"
  fi

  update_WALRUS_RELAY_PROCESS_PID_var
  echo "walrus-upload-relay started (process pid $WALRUS_RELAY_PROCESS_PID)"
}
export -f start_walrus_relay_process

stop_walrus_relay_process() {
  update_WALRUS_RELAY_PROCESS_PID_var

  if [ -n "$WALRUS_RELAY_PROCESS_PID" ]; then
    local _WORKDIR_PREFIX
    case "$WORKDIR" in
      "testnet") _WORKDIR_PREFIX="t" ;;
      "mainnet") _WORKDIR_PREFIX="m" ;;
      *) _WORKDIR_PREFIX="${WORKDIR:0:1}" ;;  # fallback: first char
    esac

    echo "Stopping ${_WORKDIR_PREFIX}walrus-upload-relay (PID $WALRUS_RELAY_PROCESS_PID)"
    kill "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true

    # Wait for process to terminate (up to 10 seconds)
    local count=0
    while [ $count -lt 10 ] && kill -0 "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null; do
      sleep 1
      count=$((count + 1))
    done

    # Force kill if still running
    if kill -0 "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null; then
      echo "Force killing ${_WORKDIR_PREFIX}walrus-upload-relay process"
      kill -9 "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true
    fi

    unset WALRUS_RELAY_PROCESS_PID
  fi

  # Note: We don't cleanup the workdir-specific symlink as it's permanent
  # The repair function will maintain it across updates
}
export -f stop_walrus_relay_process

update_WALRUS_RELAY_PROCESS_PID_var() {
  # Use the same portable approach as other Suibase processes
  local _WORKDIR_PREFIX
  case "$WORKDIR" in
    "testnet") _WORKDIR_PREFIX="t" ;;
    "mainnet") _WORKDIR_PREFIX="m" ;;
    *) _WORKDIR_PREFIX="${WORKDIR:0:1}" ;;  # fallback: first char
  esac
  local _WORKDIR_BINARY="$WORKDIRS/$WORKDIR/bin/${_WORKDIR_PREFIX}walrus-upload-relay"

  # Use get_process_pid for portable process detection (same as sui-faucet)
  local _PID
  _PID=$(get_process_pid "$_WORKDIR_BINARY")

  if [ "$_PID" = "NULL" ]; then
    unset WALRUS_RELAY_PROCESS_PID
  else
    export WALRUS_RELAY_PROCESS_PID="$_PID"
  fi
}
export -f update_WALRUS_RELAY_PROCESS_PID_var

walrus_relay_stats() {
  # Display walrus relay statistics
  # Can be either "verbose" or "quiet"
  local verbosity=$1
  
  if ! is_walrus_supported_by_workdir; then
    if [ "$verbosity" = "verbose" ]; then
      echo "Walrus relay stats not supported for $WORKDIR"
    fi
    return
  fi

  # Check if daemon is running
  update_SUIBASE_DAEMON_PID_var
  if [ -z "$SUIBASE_DAEMON_PID" ]; then
    if [ "$verbosity" = "verbose" ]; then
      echo "Suibase daemon not running - cannot retrieve walrus relay stats"
    fi
    return
  fi

  # Call the API to get walrus relay stats with display format
  local _API_RESPONSE
  _API_RESPONSE=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"getWalrusRelayStats\",\"params\":{\"workdir\":\"$WORKDIR\",\"display\":true},\"id\":1}" \
    "http://localhost:${CFG_daemon_port:-44399}" 2>/dev/null)
  
  if [ -z "$_API_RESPONSE" ]; then
    if [ "$verbosity" = "verbose" ]; then
      echo "Failed to retrieve walrus relay stats"
    fi
    return
  fi

  # Check for API errors
  if echo "$_API_RESPONSE" | jq -e '.error' >/dev/null 2>&1; then
    if [ "$verbosity" = "verbose" ]; then
      local _ERROR_MSG
      _ERROR_MSG=$(echo "$_API_RESPONSE" | jq -r '.error.message // "Unknown error"')
      echo "Error retrieving walrus relay stats: $_ERROR_MSG"
    fi
    return
  fi

  # Extract and display the formatted output
  if [ "$verbosity" = "verbose" ]; then
    local _DISPLAY_TEXT
    _DISPLAY_TEXT=$(echo "$_API_RESPONSE" | jq -r '.result.display // ""')
    if [ -n "$_DISPLAY_TEXT" ]; then
      echo "$_DISPLAY_TEXT"
    else
      echo "No display data available"
    fi
  fi
}
export -f walrus_relay_stats

walrus_relay_clear_stats() {
  # Clear walrus relay statistics
  # Can be either "verbose" or "quiet"
  local verbosity=$1
  
  if ! is_walrus_supported_by_workdir; then
    if [ "$verbosity" = "verbose" ]; then
      echo "Walrus relay stats not supported for $WORKDIR"
    fi
    return
  fi

  # Check if daemon is running
  update_SUIBASE_DAEMON_PID_var
  if [ -z "$SUIBASE_DAEMON_PID" ]; then
    if [ "$verbosity" = "verbose" ]; then
      echo "Suibase daemon not running - cannot clear walrus relay stats"
    fi
    return
  fi

  # Call the API to reset walrus relay stats
  local _API_RESPONSE
  _API_RESPONSE=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"resetWalrusRelayStats\",\"params\":{\"workdir\":\"$WORKDIR\"},\"id\":1}" \
    "http://localhost:${CFG_daemon_port:-44399}" 2>/dev/null)
  
  if [ -z "$_API_RESPONSE" ]; then
    if [ "$verbosity" = "verbose" ]; then
      echo "Failed to clear walrus relay stats"
    fi
    return
  fi

  # Check for API errors
  if echo "$_API_RESPONSE" | jq -e '.error' >/dev/null 2>&1; then
    if [ "$verbosity" = "verbose" ]; then
      local _ERROR_MSG
      _ERROR_MSG=$(echo "$_API_RESPONSE" | jq -r '.error.message // "Unknown error"')
      echo "Error clearing walrus relay stats: $_ERROR_MSG"
    fi
    return
  fi

  # Check if reset was successful
  local _RESULT_STATUS
  _RESULT_STATUS=$(echo "$_API_RESPONSE" | jq -r '.result.result // "false"')
  
  if [ "$verbosity" = "verbose" ]; then
    if [ "$_RESULT_STATUS" = "true" ]; then
      echo "Walrus relay stats cleared successfully"
    else
      local _INFO_MSG
      _INFO_MSG=$(echo "$_API_RESPONSE" | jq -r '.result.info // "Unknown error"')
      echo "Failed to clear walrus relay stats: $_INFO_MSG"
    fi
  fi
}
export -f walrus_relay_clear_stats