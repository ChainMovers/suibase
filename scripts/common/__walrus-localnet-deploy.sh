# shellcheck shell=bash

# Nodeless localnet Walrus deploy (Layer A).
#
# Publishes the Walrus Move packages to the *running* localnet Sui, sets up an
# N=1 deterministic committee whose BLS key we hold (for off-node certify),
# funds the WAL exchange, and writes:
#   - workdirs/localnet/config-default/walrus-config.yaml   (walrus CLI compatible: ids + rpc + wallet)
#   - workdirs/localnet/config-default/walrus-localnet.yaml  (suibase descriptor: package id + held committee key + chain id)
#
# NO storage nodes are started (nodeless). Real Blob/Storage objects + held-key
# certify happen on the localnet Sui; bytes are served from the filesystem by
# the WalrusStore client. See docs/dev/LOCALNET_WALRUS_PLAN.md.
#
# You must source __globals.sh before this file.

# Resolve the suibase-owned setup binary that does publish + off-node stake +
# config/descriptor writing (rust/walrus-store, built like the daemon on dev).
# Falls back to the dev build location while the binary pipeline is not wired.
update_WALRUS_LOCALNET_SETUP_BIN_var() {
  WALRUS_LOCALNET_SETUP_BIN=""
  local _candidates=(
    "$WORKDIRS/common/bin/walrus-localnet-deploy"
    "$SUIBASE_DIR/rust/walrus-store/target/release/walrus-localnet-deploy"
  )
  local _c
  for _c in "${_candidates[@]}"; do
    if [ -f "$_c" ]; then
      WALRUS_LOCALNET_SETUP_BIN="$_c"
      return 0
    fi
  done
  return 1
}
export -f update_WALRUS_LOCALNET_SETUP_BIN_var

# Best-effort wait for the localnet Sui JSON-RPC to answer before deploying.
wait_for_localnet_rpc() {
  local _rpc="${1:-http://localhost:9000}"
  local _tries="${2:-30}"
  local _i
  for ((_i = 0; _i < _tries; _i++)); do
    if curl -fsS -m 2 -X POST "$_rpc" \
      -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","id":1,"method":"sui_getChainIdentifier","params":[]}' >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}
export -f wait_for_localnet_rpc

deploy_walrus_localnet() {
  local _WORKDIR="${1:-$WORKDIR}"

  # Nodeless localnet Walrus is localnet-only.
  if [ "$_WORKDIR" != "localnet" ]; then
    return 0
  fi
  if [ ! -d "$WORKDIRS/$_WORKDIR" ]; then
    return 0
  fi

  # Opt-in feature, disabled by default (mirrors walrus_relay_enabled). When
  # off, this is a no-op so default localnet start/regen is unchanged.
  if [ "${CFG_walrus_enabled:-false}" != "true" ]; then
    return 0
  fi

  local _RPC="http://localhost:9000"
  local _FAUCET="http://localhost:9123/gas"
  local _CONFIG_DEFAULT="$WORKDIRS/$_WORKDIR/config-default"
  local _WALRUS_CONFIG="$_CONFIG_DEFAULT/walrus-config.yaml"
  local _DESCRIPTOR="$_CONFIG_DEFAULT/walrus-localnet.yaml"
  local _CONTRACTS_DIR="$HOME/repos/walrus-reference-main/contracts"

  # The setup binary is required. While the binary pipeline is not wired, this
  # is a dev-only path (build via rust/walrus-store). Non-fatal: a missing
  # binary just means "no localnet Walrus this run".
  if ! update_WALRUS_LOCALNET_SETUP_BIN_var; then
    warn_user "walrus-localnet-deploy binary not found; skipping localnet Walrus deploy. Build rust/walrus-store to enable."
    return 0
  fi

  # The localnet Sui must be up (we publish to it).
  if ! wait_for_localnet_rpc "$_RPC"; then
    warn_user "localnet Sui RPC ($_RPC) not reachable; skipping localnet Walrus deploy."
    return 0
  fi

  # Idempotency: the deploy bin compares the live chain id against the chain id
  # recorded in the descriptor and is a no-op when they already match (so this
  # is cheap to call on every start, and re-deploys after a regen wipes the chain).
  mkdir -p "$_CONFIG_DEFAULT"
  if ! "$WALRUS_LOCALNET_SETUP_BIN" deploy \
    --rpc "$_RPC" \
    --faucet "$_FAUCET" \
    --wallet "$WORKDIRS/$_WORKDIR/config/client.yaml" \
    --contracts "$_CONTRACTS_DIR" \
    --out-config "$_WALRUS_CONFIG" \
    --out-descriptor "$_DESCRIPTOR" \
    --n-shards 1000; then
    warn_user "localnet Walrus deploy failed (non-fatal); WalrusStore localnet will be unavailable until the next successful '$_WORKDIR start'/'regen'."
    return 0
  fi

  return 0
}
export -f deploy_walrus_localnet
