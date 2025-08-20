#!/bin/bash

# Call the proper walrus binary and config file combination.

# You must source __globals.sh and __walrus_binaries.sh before __walrus-exec.sh

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
    _OPT_DEFAULT_CONFIG="--config $WORKDIRS/$WORKDIR/config/walrus-config.yaml"
  else
    echo "Overriding suibase default --config is error prone and not recommended."
    echo
    echo "If you *must* use your own config consider one of these alternatives:"
    echo "  1. Modify $WORKDIRS/$WORKDIR/config/walrus-config.yaml"
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
    echo "  1. Modify $WORKDIRS/$WORKDIR/config/walrus-config.yaml"
    echo "     for temporary changes until the next '$WORKDIR update'."
    echo
    echo "  2. Call directly $WALRUS_BIN"
    echo "     for full parameter control."
    info_exit ""
  fi

  # shellcheck disable=SC2086,SC2068
  $WALRUS_BIN $_OPT_DEFAULT_CONFIG $_OPT_DEFAULT_CONTEXT "$@"

  exit
}
export -f walrus_exec
