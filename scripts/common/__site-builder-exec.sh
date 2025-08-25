#!/bin/bash

# Call the proper site-builder binary and config file combination.

# You must source __globals.sh and __walrus_binaries.sh before __site-builder-exec.sh

site_builder_exec() {

  exit_if_workdir_not_ok

  if ! is_walrus_supported_by_workdir; then
    error_exit "This script is only for testnet and mainnet."
  fi

  # Display some suibase related info if called without any parameters.
  #DISPLAY_SUIBASE_HELP=false
  #if [ $# -eq 0 ]; then
  #  DISPLAY_SUIBASE_HELP=true
  #fi

  # All other workdir use the binary from their repo.
  _BIN="$SITE_BUILDER_BIN_DIR/site-builder"

  exit_if_walrus_binary_not_ok

  #local _SUBCOMMAND=$1

  #LAST_ARG="${*: -1}"
  #if [[ "$LAST_ARG" == "--help" || "$LAST_ARG" == "-h" ]]; then
  #  DISPLAY_SUIBASE_HELP=true
  #fi

  local _OPT_DEFAULT_CONFIG=""
  local _OPT_DEFAULT_CONTEXT=""

  if ! has_param "" "--config" "$@"; then
    _OPT_DEFAULT_CONFIG="--config $WORKDIRS/$WORKDIR/config/sites-config.yaml"
  else
    echo "Overriding suibase default --config is error prone and not recommended."
    echo
    echo "If you *must* use your own config consider one of these alternatives:"
    echo "  1. Modify $WORKDIRS/$WORKDIR/config/sites-config.yaml"
    echo "     for temporary changes until the next '$WORKDIR update'."
    echo
    echo "  2. Call directly $_BIN"
    echo "     for full parameter control."
    info_exit ""
  fi

  if ! has_param "" "--context" "$@"; then
    _OPT_DEFAULT_CONTEXT="--context $WORKDIR"
  else
    echo "Overriding suibase default --context is error prone and not recommended."
    echo
    echo "If you *must* use your own context consider one of these alternatives:"
    echo "  1. Modify $WORKDIRS/$WORKDIR/config/sites-config.yaml"
    echo "     for temporary changes until the next '$WORKDIR update'."
    echo
    echo "  2. Call directly $_BIN"
    echo "     for full parameter control."
    info_exit ""
  fi

  if has_param "" "--walrus-binary" "$@"; then
    warn_user "Overriding suibase default --walrus-binary is error prone and not recommended."
    echo "Your --walrus-binary option is being applied, but try to avoid incompatible binary mixing."
    echo
  fi

  if has_param "" "--walrus-config" "$@"; then
    warn_user "Overriding suibase default --walrus-config is error prone and not recommended."
    echo "Your --walrus-config option is being applied, but try to avoid mixing configs."
    echo
  fi

  # shellcheck disable=SC2086,SC2068
  $_BIN $_OPT_DEFAULT_CONFIG $_OPT_DEFAULT_CONTEXT "$@"

  exit
}
export -f site_builder_exec
