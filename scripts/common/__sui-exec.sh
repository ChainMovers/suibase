#!/bin/bash

# Call the proper sui binary and config file combination.

# You must source __globals.sh before __sui-exec.sh

has_param() {
  local _SHORT_OPT="$1"
  local _LONG_OPT="$2"
  # Initialize params with remainng parameters (exclude $1 and $2)
  local _PARAMS=("${@:3}")

  # If found, return true.
  for _PARAM in "${_PARAMS[@]}"; do
    if [ -z "$_PARAM" ]; then
      # Should not happen... but just in case one of $_SHORT_OPT or $_LONG_OPT is empty.
      continue
    fi

    if [[ "$_PARAM" == "$_SHORT_OPT" || "$_PARAM" == "$_LONG_OPT" ]]; then
      true
      return
    fi
  done

  false
  return
}

sui_exec() {

  exit_if_workdir_not_ok

  if [ "${CFG_network_type:?}" = "local" ]; then
    is_local=true
  else
    is_local=false
  fi

  # Display some suibase related info if called without any parameters.
  DISPLAY_SUI_BASE_HELP=false
  if [ $# -eq 0 ]; then
    DISPLAY_SUI_BASE_HELP=true
  fi

  # Identify the binary to execute
  if [ "$WORKDIR" = "cargobin" ]; then
    # Special case for cargobin workdir
    SUI_BIN="$SUI_BIN_ENV $HOME/.cargo/bin/sui"
  else
    # All other workdir use the binary from their repo.
    SUI_BIN="$SUI_BIN_ENV $SUI_BIN_DIR/sui"
  fi

  exit_if_sui_binary_not_ok

  cd_sui_log_dir

  SUI_SUBCOMMAND=$1

  LAST_ARG="${*: -1}"
  if [[ "$LAST_ARG" == "--help" || "$LAST_ARG" == "-h" ]]; then
    DISPLAY_SUI_BASE_HELP=true
  fi

  if [ "$DISPLAY_SUI_BASE_HELP" = false ] && [ "$SUI_BASE_NET_MOCK" = true ] &&
    [ "$SUI_SUBCOMMAND" != "-V" ] && [ "$SUI_SUBCOMMAND" != "--version" ]; then
    echo "<sui client mock response for test>"
    exit 0
  fi

  if [[ $SUI_SUBCOMMAND == "client" || $SUI_SUBCOMMAND == "console" ]]; then
    shift 1
    $SUI_BIN "$SUI_SUBCOMMAND" --client.config "$CLIENT_CONFIG" "$@"

    if [ "$WORKDIR" = "localnet" ]; then
      # Print a friendly warning if localnet sui process found not running.
      # Might help explain weird error messages...
      if [ "$DISPLAY_SUI_BASE_HELP" = false ]; then
        update_SUI_PROCESS_PID_var
        if [ -z "$SUI_PROCESS_PID" ]; then
          echo
          echo "Warning: localnet not running"
          echo "Do 'localnet start' to get it started."
        fi
      fi
    fi

    exit
  fi

  # Make sure 'move' subcommand have always a -p or --path parameter.
  # If none provided by the user, then use the default workdir.
  # See https://github.com/ChainMovers/suibase/issues/65
  if [[ $SUI_SUBCOMMAND == "move" ]]; then
    shift 1
    local _OPT_DEFAULT_PATH=""
    if [ $DISPLAY_SUI_BASE_HELP = "false" ]; then
      if ! has_param "-p" "--path" "$@"; then
        _OPT_DEFAULT_PATH="--path $USER_CWD"
      fi
      if ! has_param "" "--install-dir" "$@"; then
        _OPT_DEFAULT_INSTALLDIR="--install-dir $USER_CWD"
      fi
    fi

    # shellcheck disable=SC2086
    $SUI_BIN "$SUI_SUBCOMMAND" $_OPT_DEFAULT_PATH $_OPT_DEFAULT_INSTALLDIR "$@"
    # echo Doing move "$_OPT_DEFAULT_PATH" "$@"

    exit
  fi

  if [ $is_local = true ]; then
    case $SUI_SUBCOMMAND in
    "keytool")
      # Are you getting an error : The argument '--keystore-path <KEYSTORE_PATH>' was provided
      # more than once, but cannot be used multiple times?
      #
      # This is because by default lsui point to the keystore created with the localnet.
      #
      # TODO Fix this. Still default to workdirs, but allow user to override with its own --keystore-path.
      #
      shift 1
      $SUI_BIN "$SUI_SUBCOMMAND" --keystore-path "$CONFIG_DATA_DIR/sui.keystore" "$@"
      ;;
    "genesis" | "genesis-ceremony" | "start")
      # Protect the user from starting more than one sui process.
      if [[ "$2" == "--help" || "$2" == "-h" ]]; then
        $SUI_BIN "$SUI_SUBCOMMAND" --help
      fi
      setup_error "Use suibase 'localnet start' script instead"
      ;;
    "network")
      shift 1
      $SUI_BIN "$SUI_SUBCOMMAND" --network.config "$NETWORK_CONFIG" "$@"
      ;;
    *)
      # By default, just pass transparently everything to the proper sui binary.
      $SUI_BIN "$@"
      ;;
    esac
  else
    # For remote network, trap many commands that just don't make sense.
    case $SUI_SUBCOMMAND in
    "genesis" | "genesis-ceremony" | "start" | "network")
      if [[ "$2" == "--help" || "$2" == "-h" ]]; then
        $SUI_BIN "$SUI_SUBCOMMAND" --help
      fi
      setup_error "Command not appplicable to remote network"
      ;;
    *)
      # By default, just pass transparently everything to the proper sui binary.
      $SUI_BIN "$@"
      ;;
    esac
  fi

  if [ "$DISPLAY_SUI_BASE_HELP" = true ]; then
    update_ACTIVE_WORKDIR_var
    if [ -n "$ACTIVE_WORKDIR" ] && [ "$WORKDIR" = "$ACTIVE_WORKDIR" ]; then
      echo
      echo -n "asui selection: [ "
      echo_blue "$ACTIVE_WORKDIR"
      echo " ]"
    fi
  fi
}
export -f sui_exec
