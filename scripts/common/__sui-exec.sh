#!/bin/bash

# Call the proper sui binary and config file combination.

# You must source __globals.sh before __sui-exec.sh

sui_exec() {

  exit_if_workdir_not_ok;

  if [ "$CFG_network_type" = "local" ]; then
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

  exit_if_sui_binary_not_ok;

  cd_sui_log_dir;

  SUI_SUBCOMMAND=$1

  LAST_ARG="${@: -1}"
  if [[ "$LAST_ARG" == "--help" || "$LAST_ARG" == "-h" ]]; then
    DISPLAY_SUI_BASE_HELP=true
  fi

  if [ "$DISPLAY_SUI_BASE_HELP" = false ] && [ "$SUI_BASE_NET_MOCK" = true ] &&
  [ "$SUI_SUBCOMMAND" != "-V" ] && [ "$SUI_SUBCOMMAND" != "--version" ] ; then
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
        update_SUI_PROCESS_PID_var;
        if [ -z "$SUI_PROCESS_PID" ]; then
          echo
          echo "Warning: localnet not running"
          echo "Do 'localnet start' to get it started."
        fi
      fi
    fi

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
      "genesis"|"genesis-ceremony"|"start")
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
      "genesis"|"genesis-ceremony"|"start"|"network")
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
    update_ACTIVE_WORKDIR_var;
    if [ -n "$ACTIVE_WORKDIR" ] && [ "$WORKDIR" = "$ACTIVE_WORKDIR" ]; then
      echo
      echo -n "asui selection: [ "; echo_blue "$ACTIVE_WORKDIR"; echo " ]";
    fi
  fi
}
export -f sui_exec
