#!/bin/bash

# Call the Suibase Drive CLI (`sui-drive`) with the network pinned by the
# calling wrapper (`ldrive`/`tdrive`/`mdrive` -> localnet/testnet/mainnet).
#
# You must source __globals.sh before __drive-exec.sh.
#
# Like `twalrus`/`lwalrus`, the wrapper preselects the workdir so you never pass
# `--network`. Unlike walrus (a downloaded Mysten binary), `sui-drive` is BUILT
# from the suiftly-tee repo (crates/sui-drive) and PINNED per-workdir at
# `workdirs/<wd>/bin/sui-drive` — the same location walrus pins into. See
# update_DRIVE_BIN_var for the resolution order + how to (re)build the pin.
#
# The binary NEVER reads an active-set env var; `--active` stays the machine
# injection surface (DRIVE_UX_REWORK §9.3). This wrapper only pins the network.

# Resolve the sui-drive binary, mirroring the lwalrus candidate search. An
# explicit dev override wins (so `export SUIFTLY_DRIVE_BIN=…/target/release/…`
# is never silently shadowed by a stale pin), then the pins:
#   1. $SUIFTLY_DRIVE_BIN    explicit dev override (point at a target/release build)
#   2. the per-workdir pin   workdirs/<wd>/bin/sui-drive   (what `<wd> update` installs)
#   3. a common pin          workdirs/common/bin/sui-drive
update_DRIVE_BIN_var() {
  DRIVE_BIN=""
  local _candidates=(
    "${SUIFTLY_DRIVE_BIN:-}"
    "$WORKDIRS/$WORKDIR/bin/sui-drive"
    "$WORKDIRS/common/bin/sui-drive"
  )
  local _c
  for _c in "${_candidates[@]}"; do
    if [ -n "$_c" ] && [ -f "$_c" ] && [ -x "$_c" ]; then
      DRIVE_BIN="$_c"
      return 0
    fi
  done
  return 1
}
export -f update_DRIVE_BIN_var

drive_exec() {

  exit_if_workdir_not_ok

  # Drive targets localnet/testnet/mainnet (unlike walrus, which is testnet/
  # mainnet only). devnet and the meta workdirs have no Drive.
  case "$WORKDIR" in
  localnet | testnet | mainnet) ;;
  *)
    error_exit "$DRIVE_SCRIPT is only for localnet, testnet and mainnet."
    ;;
  esac

  if ! update_DRIVE_BIN_var; then
    error_exit "\
sui-drive binary not found for $WORKDIR.

The Drive CLI is built from the suiftly-tee repo and pinned per-workdir. Build
and pin it, then retry:

  cd <suiftly-tee>/crates/sui-drive
  cargo build --release
  mkdir -p $WORKDIRS/$WORKDIR/bin
  cp target/release/sui-drive $WORKDIRS/$WORKDIR/bin/sui-drive

Or point SUIFTLY_DRIVE_BIN at a build:
  export SUIFTLY_DRIVE_BIN=<suiftly-tee>/crates/sui-drive/target/release/sui-drive"
  fi

  # The binary name is the network for tdrive/mdrive, but ldrive is not in the
  # binary's argv[0] sniff, and the pinned file is named `sui-drive` regardless —
  # so always pin the network explicitly (twalrus passes --context the same way).
  # shellcheck disable=SC2068
  "$DRIVE_BIN" --network "$WORKDIR" "$@"
  exit $?
}
export -f drive_exec
