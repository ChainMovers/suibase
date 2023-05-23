#!/bin/bash

# You must source __globals.sh before __sui-faucet-process.sh

start_sui_faucet_process() {

  # success/failure is reflected by the SUI_FAUCET_PROCESS_PID var.
  # noop if the process is already started.
  if [ "${CFG_sui_faucet_enabled:?}" != "true" ]; then
    return
  fi

  exit_if_sui_binary_not_ok;

  update_SUI_FAUCET_PROCESS_PID_var;
  if [ -z "$SUI_FAUCET_PROCESS_PID" ]; then
    echo "Starting $WORKDIR faucet"

    mkdir -p "$CONFIG_DATA_DIR/faucet.wal"

    if $SUI_BASE_NET_MOCK; then
      export SUI_FAUCET_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
    else
      env SUI_CONFIG_DIR="$WORKDIRS/$WORKDIR/faucet" "$SUI_BIN_DIR/sui-faucet" \
          --amount "${CFG_sui_faucet_coin_value:?}" \
          --host-ip "${CFG_sui_faucet_host_ip:?}" \
          --max-request-per-second "${CFG_sui_faucet_max_request_per_second:?}" \
          --num-coins "${CFG_sui_faucet_num_coins:?}" \
          --port "${CFG_sui_faucet_port:?}" \
          --request-buffer-size "${CFG_sui_faucet_request_buffer_size:?}" \
          --wallet-client-timeout-secs "${CFG_sui_faucet_client_timeout_secs:?}" \
          --write-ahead-log "$CONFIG_DATA_DIR/faucet.wal" >& "$CONFIG_DATA_DIR/sui-faucet-process.log" &
    fi

    # Loop until confirms can connect, or exit if that takes more than 20 seconds.
    end=$((SECONDS+20))
    ALIVE=false
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      # If it returns an error about "JSON" then it is alive!
      if $SUI_BASE_NET_MOCK; then
        CHECK_ALIVE="imagine some JSON here"
      else
        CHECK_ALIVE=$(curl -s --location \
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
    done

    # Just UI aesthetic newline for when there was "." printed.
    if [ "$AT_LEAST_ONE_SECOND" = true ]; then
      echo
    fi

    # Act on success/failure of the sui process responding to "sui client".
    if [ "$ALIVE" = false ]; then
      echo "sui-faucet process not responding. Try again? (may be the host is too slow?)."
      exit;
    fi

    update_SUI_FAUCET_PROCESS_PID_var;
    echo "faucet started (process pid $SUI_FAUCET_PROCESS_PID)"
  fi
}
export -f start_sui_faucet_process

stop_sui_faucet_process() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUI_FAUCET_PROCESS_PID_var;
  if [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
    echo "Stopping faucet (process pid $SUI_FAUCET_PROCESS_PID)"

    if $SUI_BASE_NET_MOCK; then
      unset SUI_FAUCET_PROCESS_PID
    else
      if [[ $(uname) == "Darwin" ]]; then
        kill -9 "$SUI_FAUCET_PROCESS_PID"
      else
        skill -9 "$SUI_FAUCET_PROCESS_PID"
      fi
    fi

    # Make sure it is dead.
    end=$((SECONDS+15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_SUI_FAUCET_PROCESS_PID_var;
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
  _PID=$(get_process_pid sui-faucet);
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

  local _ALL_ADDRS=()

  for option in $1; do
    case $option in
    -j|--json)
      if $_OPT_JSON; then setup_error "error: Option '$option' specified more than once"; fi
      _OPT_JSON=true ;;
    *)
      # It has to be either one or more "0x" or a single "all"
      local _ADDR
      _ADDR=$( echo "$option" | tr '[:upper:]' '[:lower:]' | tr -d "[:blank:]" )
      case $_ADDR in
      all|-all|--all)
        if (( ${#_ALL_ADDRS[@]} )); then setup_error "error: Can't mix option 'all' with enumerating address"; fi
        if $_OPT_ALL; then setup_error "error: Option '$_ADDR' specified more than once"; fi
        _OPT_ALL=true ;;
      0x*)
        if $_OPT_ALL; then setup_error "error: Can't mix option 'all' with enumerating address"; fi
        exit_if_not_valid_sui_address "$_ADDR";
        _ALL_ADDRS+=( "$_ADDR" )
      ;;
      *) setup_error "Invalid hexadecimal address [$_ADDR]" ;;
      esac ;; # address field parsing
    esac # outer options parsing
  done

  if [ $_OPT_ALL = false ] && [ ${#_ALL_ADDRS[@]} -eq 0 ]; then
    cd_sui_log_dir;
    echo "http://${CFG_sui_faucet_host_ip:?}:${CFG_sui_faucet_port:?}"
    local _FAUCET_ADDR
    _FAUCET_ADDR=$("$SUI_BIN_DIR/sui" client --client.config "$WORKDIRS/$WORKDIR/faucet/client.yaml" active-address)
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
    _RESP=$($SUI_EXEC client addresses | grep "0x" | awk '{print $1}')

    while IFS= read -r line
    do
      exit_if_not_valid_sui_address "$line";
     _ALL_ADDRS+=( "$line" )
    done < <(printf '%s\n' "$_RESP")

    echo "${#_ALL_ADDRS[@]} addresses found."
  fi

  if ! [ -x "$(command -v curl)" ]; then
    setup_error 'error: RPC to faucet requires curl to be installed.'
  fi

  for _addr in "${_ALL_ADDRS[@]}"; do
    local _RESP

    _RESP=$(curl -s --location \
    --request POST "http://${CFG_sui_faucet_host_ip:?}:${CFG_sui_faucet_port:?}/gas" \
    --header "Content-Type: application/json" \
    --data-raw "{ \"FixedAmountRequest\": {\"recipient\": \"$_addr\"}}")

    _RESP_CLEAN=$(echo "$_RESP" | tr '[:upper:]' '[:lower:]' | tr -d '[:blank:]' | tr -d '_')
    # Check for 3 confirmations of success.
    local _ERROR_ID
    (( _ERROR_ID=0 ))
    if [[ "$_RESP_CLEAN" != *"transferredgasobjects"* ]]; then
      (( _ERROR_ID += 1 ))
    fi

    if [[ "$_RESP_CLEAN" != *"transfertxdigest"* ]]; then
      (( _ERROR_ID += 10 ))
    fi

    if [[ "$_RESP_CLEAN" != *"\"error\":null"* ]]; then
      (( _ERROR_ID += 100 ))
    fi

    _N_COINS=$(count_coins "$_RESP_CLEAN")

    if [ $_ERROR_ID -eq 0 ] && [ -n "$_N_COINS" ] && [ "$_N_COINS" != "0" ]; then
      if [ "$_N_COINS" = "1" ]; then
        echo "Sent $_N_COINS coin to $_addr"
      else
        echo "Sent $_N_COINS coins to $_addr"
      fi
    else
      echo "Error ($_ERROR_ID): Sending coins to $_addr failed. Details:"
      echo "$_RESP"
      exit 1
    fi
  done
}

count_coins() {
  local _STR=$1
  local _SEP="amount"
  local _COUNT

  (( _COUNT=0 ))
  _TMP=${_STR//"$_SEP"/$'\2'}
  IFS=$'\2' read -r -a arr <<< "$_TMP"
  for _substr in "${arr[@]}" ; do
    #echo "<$_substr>"
    (( ++_COUNT ))
  done
  unset IFS
  if [ $_COUNT -ge 2 ]; then
   (( --_COUNT ))
  fi

  echo "$_COUNT"
}

