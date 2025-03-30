#!/bin/bash

# Call the proper walrus binary and config file combination.

# You must source __globals.sh and __walrus_binaries.sh before __walrus-exec.sh

CANONICAL_ARGS=()
update_CANONICAL_ARGS_var() {
  local _ARGS=("$@")
  local _ARGS_IDX=0
  local _ARGS_LEN=${#_ARGS[@]}
  CANONICAL_ARGS=()
  while [[ $_ARGS_IDX -lt $_ARGS_LEN ]]; do

    local _item=${_ARGS[_ARGS_IDX]}

    # Handle replacement potentially identified by has_move_package_path().
    #if [ "$USER_DEFINED_PACKAGE_PATH_NEEDS_FIXING" = "true" ]; then
    #  if [ "$_item" = "$USER_DEFINED_PACKAGE_PATH_ORIGINAL" ]; then
    #    _item="$USER_DEFINED_PACKAGE_PATH_CANONICAL"
    #  fi
    #fi

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

walrus_exec() {

  exit_if_workdir_not_ok

  if [ "$WORKDIR" != "testnet" ] && [ "$WORKDIR" != "mainnet" ]; then
    error_exit "This script is only for testnet and mainnet."
  fi

  # Display some suibase related info if called without any parameters.
  #DISPLAY_SUIBASE_HELP=false
  #if [ $# -eq 0 ]; then
  #  DISPLAY_SUIBASE_HELP=true
  #fi

  # All other workdir use the binary from their repo.
  WALRUS_BIN="$WALRUS_BIN_DIR/walrus"

  exit_if_walrus_binary_not_ok

  #local _SUBCOMMAND=$1

  #LAST_ARG="${*: -1}"
  #if [[ "$LAST_ARG" == "--help" || "$LAST_ARG" == "-h" ]]; then
  #  DISPLAY_SUIBASE_HELP=true
  #fi

  local _OPT_DEFAULT_CONFIG=""
  local _OPT_DEFAULT_CONTEXT=""

  if ! has_param "" "--config" "$@"; then
    _OPT_DEFAULT_CONFIG="--config $WORKDIRS/$WORKDIR/config/client_config.yaml"
  else
    echo "Overriding suibase default --config is error prone and not recommended."
    echo
    echo "If you *must* use your own config consider one of these alternatives:"
    echo "  1. Modify $WORKDIRS/$WORKDIR/config/client_config.yaml"
    echo "     for temporary changes until the next '$WORKDIR update'."
    echo
    echo "  2. Call directly $WALRUS_BIN"
    echo "     for full parameter control."
    info_exit ""
  fi

  if ! has_param "" "--context" "$@"; then
    _OPT_DEFAULT_CONTEXT="--context $WORKDIR"
  else
    echo "Overriding suibase default --context is error prone and not recommended."
    echo
    echo "If you *must* use your own context consider one of these alternatives:"
    echo "  1. Modify $WORKDIRS/$WORKDIR/config/client_config.yaml"
    echo "     for temporary changes until the next '$WORKDIR update'."
    echo
    echo "  2. Call directly $WALRUS_BIN"
    echo "     for full parameter control."
    info_exit ""
  fi

  # Resolve user specified relative paths to be absolute (make them relative to $USER_CWD).
  update_CANONICAL_ARGS_var "$@"

  # shellcheck disable=SC2086,SC2068
  $WALRUS_BIN $_OPT_DEFAULT_CONFIG $_OPT_DEFAULT_CONTEXT ${CANONICAL_ARGS[@]}

  exit
}
export -f walrus_exec
