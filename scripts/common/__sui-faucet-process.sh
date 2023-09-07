#!/bin/bash

# You must source __globals.sh before __sui-faucet-process.sh

start_sui_faucet_process() {

  # success/failure is reflected by the SUI_FAUCET_PROCESS_PID var.
  # noop if the process is already started.
  if [ "${CFG_sui_faucet_enabled:?}" != "true" ]; then
    return
  fi

  exit_if_sui_binary_not_ok

  update_SUI_FAUCET_PROCESS_PID_var
  if [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
    return
  fi

  echo "Starting $WORKDIR faucet"

  mkdir -p "$CONFIG_DATA_DIR/faucet.wal"

  # Try up to 3 times to start the process.
  end=$((SECONDS + 30))
  ALIVE=false
  AT_LEAST_ONE_SECOND=false
  for _i in {1..3}; do
    if $SUI_BASE_NET_MOCK; then
      export SUI_FAUCET_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
    else
      rm -f "$CONFIG_DATA_DIR/sui-faucet-process.log"
      env SUI_CONFIG_DIR="$WORKDIRS/$WORKDIR/faucet" "$SUI_BIN_DIR/sui-faucet" \
        --amount "${CFG_sui_faucet_coin_value:?}" \
        --host-ip "${CFG_sui_faucet_host_ip:?}" \
        --max-request-per-second "${CFG_sui_faucet_max_request_per_second:?}" \
        --num-coins "${CFG_sui_faucet_num_coins:?}" \
        --port "${CFG_sui_faucet_port:?}" \
        --request-buffer-size "${CFG_sui_faucet_request_buffer_size:?}" \
        --wallet-client-timeout-secs "${CFG_sui_faucet_client_timeout_secs:?}" \
        --write-ahead-log "$CONFIG_DATA_DIR/faucet.wal" >&"$CONFIG_DATA_DIR/sui-faucet-process.log" &
    fi

    # Loop until confirms can connect, or exit if takes too much time.
    while [ $SECONDS -lt $end ]; do
      # If it returns an error about "JSON" then it is alive!
      if $SUI_BASE_NET_MOCK; then
        CHECK_ALIVE="imagine some JSON here"
      else
        CHECK_ALIVE=$(curl -x "" -s --location \
          --request POST "http://${CFG_sui_faucet_host_ip:?}:${CFG_sui_faucet_port:?}/gas" \
          --header 'Content-Type: application/json' --data-raw '{Bad Request}' | grep "JSON")
      fi
      if [ -n "$CHECK_ALIVE" ]; then
        ALIVE=true
        break
      else
        echo -n "."
        sleep 1
        AT_LEAST_ONE_SECOND=true
      fi
      # Detect if should do a retry at starting it. This happen if the faucet
      # has difficulty to open its sockets. For unknown reason, it sometimes fail.
      if grep -q "HTTP error: error trying to connect" "$CONFIG_DATA_DIR/sui-faucet-process.log"; then
        # Sleep 5 seconds before retrying.
        for _j in {1..5}; do
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
    echo "sui-faucet process not responding. Try again? (may be the host is too slow?)."
    exit
  fi

  update_SUI_FAUCET_PROCESS_PID_var
  echo "faucet started (process pid $SUI_FAUCET_PROCESS_PID)"
}
export -f start_sui_faucet_process

stop_sui_faucet_process() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUI_FAUCET_PROCESS_PID_var
  if [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
    echo "Stopping faucet (process pid $SUI_FAUCET_PROCESS_PID)"

    if $SUI_BASE_NET_MOCK; then
      unset SUI_FAUCET_PROCESS_PID
    else
      kill -s SIGTERM "$SUI_FAUCET_PROCESS_PID"
    fi

    # Make sure it is dead.
    end=$((SECONDS + 15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_SUI_FAUCET_PROCESS_PID_var
      if [ -z "$SUI_FAUCET_PROCESS_PID" ]; then
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

    if [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
      setup_error "sui-faucet process pid=$SUI_FAUCET_PROCESS_PID still running. Try again, or stop (kill) the process yourself before proceeding."
    fi
  fi
}
export -f stop_sui_faucet_process

update_SUI_FAUCET_VERSION_var() {
  # --version not supported by sui-faucet yet. Always mock it.
  export SUI_FAUCET_VERSION="0.0.0"
}
export -f update_SUI_FAUCET_VERSION_var

update_SUI_FAUCET_PROCESS_PID_var() {
  if $SUI_BASE_NET_MOCK; then return; fi

  local _PID
  _PID=$(get_process_pid sui-faucet)
  if [ "$_PID" = "NULL" ]; then
    unset SUI_FAUCET_PROCESS_PID
  else
    export SUI_FAUCET_PROCESS_PID="$_PID"
  fi
}
export -f update_SUI_FAUCET_PROCESS_PID_var

faucet_command() {
  local _OPT_JSON _OPT_ALL _OPT_ADDR

  _OPT_JSON=false
  _OPT_ALL=false
  _OPT_HELP=false

  local _ALL_ADDRS=()

  for option in $1; do
    case $option in
    -j | --json)
      if $_OPT_JSON; then error_exit "Option '$option' specified more than once"; fi
      _OPT_JSON=true
      ;;
    -h | --help | help | -help | --h | -\?)
      _OPT_HELP=true
      ;;
    *)
      # It has to be either one or more "0x" or a single "all"
      local _ADDR
      _ADDR=$(echo "$option" | tr '[:upper:]' '[:lower:]' | tr -d "[:blank:]")
      case $_ADDR in
      all | -all | --all)
        if ((${#_ALL_ADDRS[@]})); then error_exit "Can't mix option 'all' with enumerating address"; fi
        if $_OPT_ALL; then error_exit "Option '$_ADDR' specified more than once"; fi
        _OPT_ALL=true
        ;;
      0x*)
        if $_OPT_ALL; then error_exit "Can't mix option 'all' with enumerating address"; fi
        exit_if_not_valid_sui_address "$_ADDR"
        _ALL_ADDRS+=("$_ADDR")
        ;;
      *) error_exit "Invalid hexadecimal address [$option]" ;;
      esac
      ;; # address field parsing
    esac # outer options parsing
  done

  local _SHOW_USEAGE=false
  if [ $_OPT_HELP = true ]; then
    _SHOW_USEAGE=true
  else
    if [ $_OPT_ALL = false ] && [ ${#_ALL_ADDRS[@]} -eq 0 ]; then
      _SHOW_USEAGE=true
    fi
  fi

  if [ $_SHOW_USEAGE = true ]; then
    cd_sui_log_dir
    echo "http://${CFG_sui_faucet_host_ip:?}:${CFG_sui_faucet_port:?}"
    update_ACTIVE_ADDRESS_var "$SUI_BIN_DIR/sui" "$WORKDIRS/$WORKDIR/faucet/client.yaml"
    local _FAUCET_ADDR=$ACTIVE_ADDRESS
    # Display address only if looking coherent.
    if [[ "$_FAUCET_ADDR" == *"0x"* ]]; then
      echo "Address: $_FAUCET_ADDR"
      local _FAUCET_GAS
      _FAUCET_GAS=$($SUI_BIN_ENV "$SUI_BIN_DIR/sui" client --client.config "$WORKDIRS/$WORKDIR/faucet/client.yaml" gas | awk '{ sum += $3} END { print sum }')
      # Display balance only if looking coherent.
      if ! [[ "$_FAUCET_GAS" =~ ^[^0-9]+$ ]]; then
        echo "Balance: $_FAUCET_GAS"
      fi
    fi
    echo
    echo "Usage: $WORKDIR faucet <ADDRESS 1> ... <ADDRESS n> | all"
    exit
  fi

  if $_OPT_ALL; then
    local _RESP
    # TODO Replace this with --json once there is a bash script way to parse JSON arrays.
    _RESP=$($SUI_EXEC client addresses | grep -v "activeAddress" | grep "0x")

    while IFS= read -r line; do
      if [[ "$line" =~ 0x[[:xdigit:]]+ ]]; then
        local _HEX_ADDR="${BASH_REMATCH[0]}"
        # No need to do: exit_if_not_valid_sui_address "$_HEX_ADDR"
        # The output was from the sui binary and very likely valid.
        # shelcheck disable=SC2076
        if ! [[ " ${_ALL_ADDRS[*]} " == *" ${_HEX_ADDR} "* ]]; then
          _ALL_ADDRS+=("$_HEX_ADDR")
        fi
      fi
    done < <(printf '%s\n' "$_RESP")

    echo "${#_ALL_ADDRS[@]} addresses found."
  fi

  if ! [ -x "$(command -v curl)" ]; then
    setup_error 'RPC to faucet requires curl to be installed.'
  fi

  for _addr in "${_ALL_ADDRS[@]}"; do
    local _RESP

    _RESP=$(curl -x "" -s --location \
      --request POST "http://${CFG_sui_faucet_host_ip:?}:${CFG_sui_faucet_port:?}/gas" \
      --header "Content-Type: application/json" \
      --data-raw "{ \"FixedAmountRequest\": {\"recipient\": \"$_addr\"}}")

    _RESP_CLEAN=$(echo "$_RESP" | tr '[:upper:]' '[:lower:]' | tr -d '[:blank:]' | tr -d '_')
    # Check for 3 confirmations of success.
    local _ERROR_ID
    ((_ERROR_ID = 0))
    if [[ "$_RESP_CLEAN" != *"transferredgasobjects"* ]]; then
      ((_ERROR_ID += 1))
    fi

    if [[ "$_RESP_CLEAN" != *"transfertxdigest"* ]]; then
      ((_ERROR_ID += 10))
    fi

    if [[ "$_RESP_CLEAN" != *"\"error\":null"* ]]; then
      ((_ERROR_ID += 100))
    fi

    _N_COINS=$(count_coins "$_RESP_CLEAN")

    if [ $_ERROR_ID -eq 0 ] && [ -n "$_N_COINS" ] && [ "$_N_COINS" != "0" ]; then
      if [ "$_N_COINS" = "1" ]; then
        echo "Sent $_N_COINS coin to $_addr"
      else
        echo "Sent $_N_COINS coins to $_addr"
      fi
    else
      echo "Error ($_ERROR_ID). Details:"
      echo "$_RESP"
      error_exit "Sending coins to $_addr failed."
    fi
  done
}

count_coins() {
  local _STR=$1
  local _SEP="amount"
  local _COUNT

  ((_COUNT = 0))
  _TMP=${_STR//"$_SEP"/$'\2'}
  IFS=$'\2' read -r -a arr <<<"$_TMP"
  for _substr in "${arr[@]}"; do
    #echo "<$_substr>"
    ((++_COUNT))
  done
  unset IFS
  if [ $_COUNT -ge 2 ]; then
    ((--_COUNT))
  fi

  echo "$_COUNT"
}
