# shellcheck shell=bash

# You must source __globals.sh before __autocoins.sh

# One file provides the status data:
#   ~/suibase/workdirs/common/autocoins/status.yaml
#
# Note: Even if in common, it is assumed that most fields are for testnet. More fields for, say, devnet
#       could be added later (e.g. devnet_status, devnet_deposit_total, etc).
#
# testnet status.yaml format is:
#   status: "DISABLED", "INITIALIZING", "OK", "RECOVERING", "DOWN"
#   last_error: "error message"
#   last_warning: "warning message"
#   deposit_address: "0x1234cccc...1726374abcd" <- 64 chars, but shorten for display to '(0x1234..abcd)'
#   deposit_total: 5   <- Displayed as ">999" if greater than 999
#   percent_downloaded: 100  <- From 0 to 100
#   last_verification_attempt: 79836235 <- Unix timestamp in seconds
#   last_verification_ok: 79836235 <- Unix timestamp in seconds
#   last_verification_failed: 79830123 <- Unix timestamp in seconds
#   day_offset: 283712 <- Offset in seconds during the day
#
# The status.yaml is loaded here when __autocoins.sh is sourced, in same way suibase.yaml is loaded
# when __globals.sh is sourced. The file is read-only and written by the suibase-daemon only.
#

# Supported functions:
#      - autocoins_status display info base on the status.yaml file.
#          NOT RUNNING is displayed when suibase-daemon is detected not running and
#          it overrides the status from the status.yaml file.
#
#      - autocoins_enable
#          Modify the ~/suibase/workdirs/testnet/suibase.yaml file by
#          adding or updating 'autocoins_enabled: true'.
#
#      - autocoins_disable
#          Modify the ~/suibase/workdirs/testnet/suibase.yaml file by
#          adding or updating 'autocoins_enabled: false'.
#
#      - autocoins_set_address "$AUTOCOINS_ADDRESS"
#          Modify the ~/suibase/workdirs/testnet/suibase.yaml file by
#          adding or updating 'autocoins_address: $AUTOCOINS_ADDRESS'.
#
#      - autocoins_purge_data
#          Delete all the contents of the ~/suibase/workdirs/common/autocoins/data directory.

AUTOCOINS_DIR="$WORKDIRS/common/autocoins"
AUTOCOINS_STATUS_FILE="$AUTOCOINS_DIR/status.yaml"
AUTOCOINS_DATA_DIR="$AUTOCOINS_DIR/data"
AUTOCOINS_SUIBASE_YAML="$WORKDIRS/$WORKDIR/suibase.yaml"

update_autocoins_status_yaml() {

  # Parse the status.yaml file
  if [ -f "$AUTOCOINS_STATUS_FILE" ]; then
    eval "$(parse_yaml "$AUTOCOINS_STATUS_FILE" "ACOINS_")"
  else
    # Use default values if file doesn't exist yet...
    ACOINS_tstatus="INITIALIZING"
    ACOINS_tenabled="false"
    ACOINS_tsui_address=""
    ACOINS_tsui_deposit=0
    ACOINS_percent_downloaded=0
    # shellcheck disable=SC2034
    ACOINS_last_verification_attempt=0
    # shellcheck disable=SC2034
    ACOINS_last_verification_ok=0
    # shellcheck disable=SC2034
    ACOINS_last_verification_failed=0
    ACOINS_last_error=""
    ACOINS_last_warning=""
    # shellcheck disable=SC2034
    ACOINS_day_offset=0
  fi
}

# Always try to load the yaml upon sourcing.
update_autocoins_status_yaml

# Format address to shortened format (0x123..bcd)
format_address() {
  local address=$1
  if [ -n "$address" ]; then
    local prefix="${address:0:5}"
    local suffix="${address: -3}"
    echo "($prefix..$suffix)"
  fi
}

# Format deposit count with padding
format_deposit_count() {
  local count=$1
  if [ -z "$count" ] || [ "$count" -eq 0 ]; then
    echo "(0 deposit)"
  elif [ "$count" -eq 1 ]; then
    echo "(1 deposit)"
  elif [ "$count" -gt 999 ]; then
    echo "(>999 deposits)"
  else
    echo "($count deposits)"
  fi
}

format_human_readable_size() {
  local size_bytes=$1
  local size
  local suffix

  if [ "$size_bytes" -lt 1024 ]; then
    size=$size_bytes
    suffix="bytes"
  elif [ "$size_bytes" -lt 1048576 ]; then
    size=$(( size_bytes / 1024 ))
    suffix="KB"
  elif [ "$size_bytes" -lt 1073741824 ]; then
    size=$(( size_bytes / 1048576 ))
    suffix="MB"
  else
    size=$(( size_bytes / 1073741824 ))
    suffix="GB"
  fi

  echo "$size $suffix"
}

set_deposit_address_as_needed() {
  local verbosity=$1 # Can be either "verbose" or "quiet"

  if ! grep -q "^autocoins_address:" "$AUTOCOINS_SUIBASE_YAML" || [[ "$(grep "^autocoins_address:" "$AUTOCOINS_SUIBASE_YAML" | awk '{print $2}')" == "\"\"" ]]; then
    # Try to get active address from client.yaml
    local active_address
    update_ACTIVE_ADDRESS_var "$SUI_BIN_DIR/sui" "$CLIENT_CONFIG"
    # shellcheck disable=SC2153
    active_address="$ACTIVE_ADDRESS"

    if [ -n "$active_address" ]; then
      autocoins_set_address "$verbosity" "$active_address"
    elif [ "$verbosity" = "verbose" ]; then
      warn_user "No active address found in client.yaml"
      echo "Please set an address with 'testnet autocoins set <ADDRESS>'"
    fi
  fi
}

autocoins_echo_status_color() {
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
  #  RECOVERING
  #
  # Red
  #  DOWN
  #  NOT RUNNING
  case "$_status" in
  "INITIALIZING" | "OK")
    echo_blue "$_status"
    ;;
  "DOWNLOADING")
    echo_yellow "$_status"
    ;;
  "DOWN" | "NOT RUNNING" | "STOPPED")
    echo_red "$_status"
    ;;
  *)
    echo -n "$_status"
    ;;
  esac
}
export -f autocoins_echo_status_color

export AUTOCOINS_STATUS="DOWN"
export AUTOCOINS_INFO=""
autocoins_status() {
  # Can be either "verbose" or "quiet"
  #
  # When "quiet" the AUTOCOINS_STATUS and AUTOCOINS_INFO variables are still updated
  # but there is no stdout output.
  #
  local verbosity=$1
  local suibase_daemon_pid=$2
  local user_request=$3

  if [ "$WORKDIR" != "testnet" ]; then
    AUTOCOINS_STATUS="DOWN"
    AUTOCOINS_INFO=""
    if [ "$verbosity" = "verbose" ]; then
      setup_error "Autocoins is only supported for testnet"
    fi
  fi

  # Check if autocoins is enabled in config
  if [ "${CFG_autocoins_enabled:?}" != "true" ]; then
    AUTOCOINS_STATUS="DISABLED"
    AUTOCOINS_INFO=""
    if [ "$verbosity" = "verbose" ]; then
      echo "Autocoins : $AUTOCOINS_STATUS $AUTOCOINS_INFO"
      echo "To enable do 'testnet autocoins enable'"
    fi
    return
  fi


  # Display the status line
  #  Autocoins : DISABLED
  #  Autocoins : OK           (0x123..bcd) (   5 deposits) 100% downloaded
  #  Autocoins : DOWNLOADING  (0x123..bcd) (   8 deposits)  90% downloaded
  #  Autocoins : DOWN         (0x123..bcd) (>999 deposits) 100% downloaded
  #  Autocoins : NOT RUNNING  (0x123..bcd) (   9 deposits) 100% downloaded
  #  Autocoins : STOPPED      (0x123..bcd) (   9 deposits) 100% downloaded
  #
  # Get the info from the status.yaml file
  AUTOCOINS_INFO="$(format_address "${ACOINS_tsui_address:-}") $(format_deposit_count "${ACOINS_tsui_deposit:-0}") ${ACOINS_percent_downloaded:-0}% downloaded"

  if [ "$user_request" = "stop" ]; then
    AUTOCOINS_STATUS="STOPPED"
  elif [ -z "$suibase_daemon_pid" ]; then
    AUTOCOINS_STATUS="NOT RUNNING"
  elif [ "${ACOINS_tenabled:-}" = "false" ]; then
    # The config was changed to enabled, but not yet refelected by the daemon.
    AUTOCOINS_STATUS="ENABLING"
  else
    AUTOCOINS_STATUS="$ACOINS_tstatus"
  fi

  if [ "$verbosity" = "verbose" ]; then
    echo -n "Autocoins: "
    autocoins_echo_status_color "$AUTOCOINS_STATUS"
    echo " $AUTOCOINS_INFO"

    # Help the user...
    if [ -z "$suibase_daemon_pid" ]; then
      echo
      echo "To run services do 'testnet start'"
      echo
    fi

    if [ -n "${ACOINS_last_error:-}" ]; then
      echo_red "Error: "
      echo "${ACOINS_last_error}"
    fi
    if [ -n "${ACOINS_last_warning:-}" ]; then
      echo_yellow "Warning: "
      echo "${ACOINS_last_warning}"
    fi
  fi
}
export -f autocoins_status


autocoins_enable() {

  local verbosity=$1

  if [ "$WORKDIR" != "testnet" ]; then
    setup_error "Autocoins is only supported for testnet"
  fi

  # Check if suibase.yaml exists
  if [ ! -f "$AUTOCOINS_SUIBASE_YAML" ]; then
    setup_error "Cannot find suibase.yaml at [$AUTOCOINS_SUIBASE_YAML]"
  fi


  set_deposit_address_as_needed "verbose"

  # Check if autocoins is already enabled
  local already_enabled=false
  if [ "${CFG_autocoins_enabled:?}" == "true" ]; then
    already_enabled=true
  else
    # Update suibase.yaml
    if grep -q "^autocoins_enabled:" "$AUTOCOINS_SUIBASE_YAML"; then
      # Replace existing line
      sed -i.bak "s/^autocoins_enabled:.*/autocoins_enabled: true/" "$AUTOCOINS_SUIBASE_YAML" && rm "$AUTOCOINS_SUIBASE_YAML.bak"
    else
      # Add new line
      echo "autocoins_enabled: true" >> "$AUTOCOINS_SUIBASE_YAML"
    fi
    # Re-load the modified config into this shell process.
    update_autocoins_status_yaml
    # Check if suibase daemon is running
    exit_if_sui_binary_not_ok
    if ! start_suibase_daemon_as_needed; then
      echo "suibase services are not running. Do '$WORKDIR start'."
    fi
  fi


  # Display appropriate message
  if [ "$already_enabled" = true ]; then
    echo "Autocoins already enabled"
  else
    echo "Autocoins is now enabled"
  fi
}
export -f autocoins_enable

autocoins_disable() {

  local verbosity=$1

  if [ "$WORKDIR" != "testnet" ]; then
    setup_error "Autocoins is only supported for testnet"
  fi

  # Check if suibase.yaml exists
  if [ ! -f "$AUTOCOINS_SUIBASE_YAML" ]; then
    setup_error "Cannot find suibase.yaml at $AUTOCOINS_SUIBASE_YAML"
  fi

  local already_disabled=false
  if [ "${CFG_autocoins_enabled:?}" != "true" ]; then
    already_disabled=true
  else
    # Update suibase.yaml only if not already disabled
    if grep -q "^autocoins_enabled:" "$AUTOCOINS_SUIBASE_YAML"; then
      # Replace existing line
      sed -i.bak "s/^autocoins_enabled:.*/autocoins_enabled: false/" "$AUTOCOINS_SUIBASE_YAML" && rm "$AUTOCOINS_SUIBASE_YAML.bak"
    else
      # Add new line
      echo "autocoins_enabled: false" >> "$AUTOCOINS_SUIBASE_YAML"
    fi
    # Re-load the modified config into this shell process.
    update_autocoins_status_yaml
  fi

  if [ "$verbosity" = "verbose" ]; then
    if [ "$already_disabled" = true ]; then
      echo "Autocoins already disabled"
    else
      echo "Autocoins is now disabled"
    fi
  fi
}
export -f autocoins_disable

autocoins_set_address() {
  local verbosity=$1
  local address=$2

  if [ "$WORKDIR" != "testnet" ]; then
    setup_error "Autocoins is only supported for testnet"
  fi

  # Check if suibase.yaml exists
  if [ ! -f "$AUTOCOINS_SUIBASE_YAML" ]; then
    setup_error "Cannot find suibase.yaml at $AUTOCOINS_SUIBASE_YAML"
  fi

  # Check if address is valid
  if ! check_is_valid_hex_pk "$address"; then
    setup_error "Invalid Sui account address: $address"
  fi

  # Update suibase.yaml
  if grep -q "^autocoins_address:" "$AUTOCOINS_SUIBASE_YAML"; then
    # Replace existing line
    sed -i.bak "s|^autocoins_address:.*|autocoins_address: \"$address\"|" "$AUTOCOINS_SUIBASE_YAML" && rm "$AUTOCOINS_SUIBASE_YAML.bak"
  else
    # Add new line
    echo "autocoins_address: \"$address\"" >> "$AUTOCOINS_SUIBASE_YAML"
  fi
}
export -f autocoins_set_address

autocoins_purge_data() {

  local verbosity=$1

  if [ "$WORKDIR" != "testnet" ]; then
    setup_error "Autocoins is only supported for testnet"
  fi

  # Check if data dir exists
  if [ ! -d "$AUTOCOINS_DATA_DIR" ]; then
    if [ "$verbosity" = "verbose" ]; then
      echo "No autocoins data directory found. Nothing to purge."
    fi
    return
  fi

  # Check if data dir is empty.
  if [ -z "$(ls -A "$AUTOCOINS_DATA_DIR")" ]; then
    if [ "$verbosity" = "verbose" ]; then
      echo "No autocoins data found. Nothing to purge."
    fi
    return
  fi

  local size_bytes
  size_bytes=$(du -sb "$AUTOCOINS_DATA_DIR" | cut -f1)
  local human_readable_size
  human_readable_size=$(format_human_readable_size "$size_bytes")

  # Confirm purge
  if [ "$verbosity" = "verbose" ]; then
    echo "Autocoins service will be disabled and all related disk space will be freed."
    echo "If you choose to re-enable later, then data will need to be re-downloaded"
    echo "and it will take at least 25 days for deposit to resume normally."
    if [ -n "$human_readable_size" ]; then
      echo
      echo "$human_readable_size will be freed."
    fi
    echo
    echo -n "Are you sure? [y/N] "
    read -r _confirm

    if [[ ! "$_confirm" =~ ^[Yy]$ ]]; then
      echo
      echo "Purge cancelled."
      return
    fi
  fi

  autocoins_disable "quiet"

  # Delete data
  rm -rf "${AUTOCOINS_DATA_DIR:?}"/*

  if [ "$verbosity" = "verbose" ]; then
    echo
    echo "Autocoins data purge completed."
  fi
}
export -f autocoins_purge_data