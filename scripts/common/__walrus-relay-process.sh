# shellcheck shell=bash

# You must source __globals.sh before __walrus-relay-process.sh

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
  
  # Get configuration values
  local _RELAY_PORT="${CFG_walrus_relay_local_port:?}"
  local _WALRUS_CONFIG="$CONFIG_DATA_DIR_DEFAULT/walrus-config.yaml"
  local _RELAY_CONFIG="$CONFIG_DATA_DIR_DEFAULT/relay-config.yaml"
  
  # Verify walrus-upload-relay binary exists
  local _RELAY_BINARY="$WORKDIRS/$WORKDIR/bin/walrus-upload-relay"
  if [ ! -f "$_RELAY_BINARY" ]; then
    setup_error "walrus-upload-relay binary not found at $_RELAY_BINARY"
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
    rm -f "$CONFIG_DATA_DIR/walrus-relay-process.log" >/dev/null 2>&1
    
    # Start walrus-upload-relay process
    "$_RELAY_BINARY" \
      --context "$WORKDIR" \
      --walrus-config "$_WALRUS_CONFIG" \
      --server-address "0.0.0.0:$_RELAY_PORT" \
      --relay-config "$_RELAY_CONFIG" \
      >"$CONFIG_DATA_DIR/walrus-relay-process.log" 2>&1 &

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
      if [ -f "$CONFIG_DATA_DIR/walrus-relay-process.log" ] && \
         grep -q "Address already in use\|failed to load\|panicked" "$CONFIG_DATA_DIR/walrus-relay-process.log"; then
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
    if [ -f "$CONFIG_DATA_DIR/walrus-relay-process.log" ]; then
      tail -10 "$CONFIG_DATA_DIR/walrus-relay-process.log"
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
    echo "Stopping walrus-upload-relay (PID $WALRUS_RELAY_PROCESS_PID)"
    kill "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true
    wait "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true
    unset WALRUS_RELAY_PROCESS_PID
  fi
}
export -f stop_walrus_relay_process

update_WALRUS_RELAY_PROCESS_PID_var() {
  # Find walrus-upload-relay process for this workdir
  local _RELAY_PORT="${CFG_walrus_relay_local_port:-}"
  if [ -z "$_RELAY_PORT" ]; then
    unset WALRUS_RELAY_PROCESS_PID
    return
  fi
  
  # Look for walrus-upload-relay process listening on the expected port
  local _PID
  _PID=$(lsof -ti ":$_RELAY_PORT" 2>/dev/null | head -1)
  
  if [ -n "$_PID" ] && kill -0 "$_PID" 2>/dev/null; then
    # Verify it's actually walrus-upload-relay
    if ps -p "$_PID" -o cmd= | grep -q "walrus-upload-relay"; then
      export WALRUS_RELAY_PROCESS_PID="$_PID"
    else
      unset WALRUS_RELAY_PROCESS_PID
    fi
  else
    unset WALRUS_RELAY_PROCESS_PID
  fi
}
export -f update_WALRUS_RELAY_PROCESS_PID_var