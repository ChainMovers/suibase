#!/bin/bash

# Call the proper sui binary and config file combination.

# You must source __globals.sh before __sui-exec.sh

has_param() {
  local _SHORT_OPT="$1"
  local _LONG_OPT="$2"
  # Initialize params with remaining parameters (exclude $1 and $2)
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

USER_DEFINED_PACKAGE_PATH_NEEDS_FIXING=false
USER_DEFINED_PACKAGE_PATH_ORIGINAL=""
USER_DEFINED_PACKAGE_PATH_CANONICAL=""
has_move_package_path() {
  USER_DEFINED_PACKAGE_PATH_NEEDS_FIXING=false
  USER_DEFINED_PACKAGE_PATH_ORIGINAL=""
  USER_DEFINED_PACKAGE_PATH_CANONICAL=""
  # Heuristic is used to minimize maintenance (likely to keep working even
  # when Mysten Labs add/remove options and the script is not yet updated).
  #
  # Logic is:
  #      If a non-option is found then consider that the path is specified.
  #      Assume all params starting with a "-" are options.
  #      Do extra skip for known options requiring its own arg.
  local _ARGS=("$@")
  local _ARGS_IDX=1 # Skip the first arg (the sub-subcommand)
  local _ARGS_LEN=${#_ARGS[@]}
  while [[ $_ARGS_IDX -lt $_ARGS_LEN ]]; do
    # echo "Processing ${_ARGS[_ARGS_IDX]}"
    _item=${_ARGS[_ARGS_IDX]}
    _ARGS_IDX=$((_ARGS_IDX + 1)) # Similar to a 'shift'. Remove a single element.
    case $_item in
    --install-dir | --default-move-flavor | --default-move-edition | --gas | --gas-budget | --upgrade-capability)
      _ARGS_IDX=$((_ARGS_IDX + 1)) # Do an extra shift for the following expected arg.
      ;;
    -*)
      # Likely to be a single option without additional arg.
      # Do nothing (just skip it).
      ;;
    *)
      # non-option found, likely to be the package_path

      if [[ $_item == /* ]]; then
        USER_DEFINED_PACKAGE_PATH_NEEDS_FIXING=false
        USER_DEFINED_PACKAGE_PATH_ORIGINAL=$_item
        USER_DEFINED_PACKAGE_PATH_CANONICAL=$_item
      else
        # Used later to fix the original path.
        USER_DEFINED_PACKAGE_PATH_NEEDS_FIXING=true
        USER_DEFINED_PACKAGE_PATH_ORIGINAL=$_item
        USER_DEFINED_PACKAGE_PATH_CANONICAL="$USER_CWD/$_item"
      fi
      true
      return
      ;;
    esac

  done

  false
  return
}
export -f has_move_package_path

CANONICAL_ARGS=()
update_CANONICAL_ARGS_var() {
  local _ARGS=("$@")
  local _ARGS_IDX=0
  local _ARGS_LEN=${#_ARGS[@]}
  CANONICAL_ARGS=()
  while [[ $_ARGS_IDX -lt $_ARGS_LEN ]]; do

    local _item=${_ARGS[_ARGS_IDX]}

    # Handle replacement potentially identified by has_move_package_path().
    if [ "$USER_DEFINED_PACKAGE_PATH_NEEDS_FIXING" = "true" ]; then
      if [ "$_item" = "$USER_DEFINED_PACKAGE_PATH_ORIGINAL" ]; then
        _item="$USER_DEFINED_PACKAGE_PATH_CANONICAL"
      fi
    fi

    # Append $item into CANONICAL_ARGS
    CANONICAL_ARGS+=("$_item")
    _ARGS_IDX=$((_ARGS_IDX + 1))

    case $_item in
    -p | --path | --install-dir)
      # Canonicalize following expected arg (a path).
      if [[ $_ARGS_IDX -lt $_ARGS_LEN ]]; then
        local _TARGET_DIR="${_ARGS[$_ARGS_IDX]}"
        if [[ $_TARGET_DIR != /* ]]; then
          # Relative path, prepend with $USER_CWD
          _TARGET_DIR="$USER_CWD/$_TARGET_DIR"
        fi
        CANONICAL_ARGS+=("$_TARGET_DIR")
        _ARGS_IDX=$((_ARGS_IDX + 1))
      fi
      ;;
    *) ;; # Do nothing.
    esac
  done
}
export -f update_CANONICAL_ARGS_var

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

    # Some client subcommands requires to compensate for when the user
    # does not specify path (and default to current dir).
    local _OPT_DEFAULT_PATH=""
    local _OPT_DEFAULT_INSTALLDIR=""
    if [[ $SUI_SUBCOMMAND == "client" ]]; then
      case $1 in
      publish | verify-source | verify-bytecode-meter | upgrade)
        if ! has_move_package_path "$@"; then
          # Compensate with adding the current directory path to the command.
          _OPT_DEFAULT_PATH="$USER_CWD"
        fi
        if ! has_param "" "--install-dir" "$@"; then
          _OPT_DEFAULT_INSTALLDIR="--install-dir $USER_CWD"
        fi
        ;;
      *) ;; # Do nothing
      esac
    fi

    # Resolve user specified relative paths to be absolute (make them relative to $USER_CWD).
    update_CANONICAL_ARGS_var "$@"

    # shellcheck disable=SC2086,SC2068
    $SUI_BIN "$SUI_SUBCOMMAND" --client.config "$CLIENT_CONFIG" ${CANONICAL_ARGS[@]} $_OPT_DEFAULT_INSTALLDIR $_OPT_DEFAULT_PATH

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
    local _OPT_DEFAULT_INSTALLDIR=""
    if [ $DISPLAY_SUI_BASE_HELP = "false" ]; then
      if ! has_param "-p" "--path" "$@"; then
        _OPT_DEFAULT_PATH="--path $USER_CWD"
      fi
      if ! has_param "" "--install-dir" "$@"; then
        _OPT_DEFAULT_INSTALLDIR="--install-dir $USER_CWD"
      fi
    fi

    # Resolve user specified relative paths to be absolute (make them relative to $USER_CWD).
    update_CANONICAL_ARGS_var "$@"

    # shellcheck disable=SC2086,SC2068
    $SUI_BIN "$SUI_SUBCOMMAND" $_OPT_DEFAULT_PATH $_OPT_DEFAULT_INSTALLDIR ${CANONICAL_ARGS[@]}
    exit
  fi

  if [ $is_local = true ]; then
    case $SUI_SUBCOMMAND in
    "keytool")
      shift 1
      # Append default --keystore-path, unless specified by the caller.
      if ! has_param "" "--keystore-path" "$@"; then
        $SUI_BIN "$SUI_SUBCOMMAND" --keystore-path "$CONFIG_DATA_DIR/sui.keystore" "$@"
      else
        $SUI_BIN "$SUI_SUBCOMMAND" "$@"
      fi
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
    "keytool")
      shift 1
      # Append default --keystore-path, unless specified by the caller.
      if ! has_param "" "--keystore-path" "$@"; then
        $SUI_BIN "$SUI_SUBCOMMAND" --keystore-path "$CONFIG_DATA_DIR/sui.keystore" "$@"
      else
        $SUI_BIN "$SUI_SUBCOMMAND" "$@"
      fi
      ;;
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
