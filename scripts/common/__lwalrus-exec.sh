#!/bin/bash

# Call the localnet 'lwalrus' binary (the localnet, walrus-shaped CLI).
#
# You must source __globals.sh before __lwalrus-exec.sh.
#
# Unlike __walrus-exec.sh (testnet/mainnet, real 'walrus' binary), this is localnet-only
# and gated on walrus_local_enabled. It does NOT relax is_walrus_supported_by_workdir
# (which intentionally stays testnet/mainnet for the real-binary path).

# Resolve the lwalrus binary, mirroring update_WALRUS_LOCALNET_SETUP_BIN_var's candidate
# search (precompiled/installed first, then a local dev build of rust/localnet-tools).
update_LWALRUS_BIN_var() {
  LWALRUS_BIN=""
  local _candidates=(
    "$WORKDIRS/common/bin/lwalrus"
    "$SUIBASE_DIR/rust/localnet-tools/target/release/lwalrus"
    "$SUIBASE_DIR/rust/localnet-tools/target/debug/lwalrus"
  )
  local _c
  for _c in "${_candidates[@]}"; do
    if [ -f "$_c" ] && [ -x "$_c" ]; then
      LWALRUS_BIN="$_c"
      return 0
    fi
  done
  return 1
}
export -f update_LWALRUS_BIN_var

lwalrus_exec() {

  exit_if_workdir_not_ok

  if [ "$WORKDIR" != "localnet" ]; then
    error_exit "lwalrus is only for localnet. Use 'twalrus'/'mwalrus' for testnet/mainnet."
  fi

  if [ "${CFG_walrus_local_enabled:-false}" != "true" ]; then
    error_exit "Localnet Walrus is disabled. Set 'walrus_local_enabled: true' in $WORKDIRS/$WORKDIR/suibase.yaml, then '$WORKDIR regen'."
  fi

  # Best-effort self-heal: build/fetch the localnet-tools binaries (incl. lwalrus)
  # if the helper is available and the binary is not present yet.
  if ! update_LWALRUS_BIN_var; then
    if type -t update_localnet_tools_bin >/dev/null 2>&1; then
      update_localnet_tools_bin "$WORKDIR"
      update_LWALRUS_BIN_var
    fi
  fi

  if [ -z "$LWALRUS_BIN" ]; then
    error_exit "lwalrus binary not found. Run '$WORKDIR update' to build/fetch the localnet tools."
  fi

  # shellcheck disable=SC2068
  "$LWALRUS_BIN" "$@"
  exit $?
}
export -f lwalrus_exec
