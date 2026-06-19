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
    "$SUIBASE_DIR/rust/walrus-store/target/debug/walrus-localnet-deploy"
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

# Fetch the precompiled "localnet-tools" asset (which bundles the walrus-localnet-deploy
# binary) from chainmovers/sui-binaries (app "localnet_tools" defined in
# scripts/defaults/consts.yaml). localnet + walrus_enabled only. Best-effort + non-fatal:
# a missing/not-yet-published asset or offline state must never abort 'localnet start';
# the deploy then falls back to a local dev build of rust/walrus-store. The install runs
# in a subshell so that even a hard error inside the app machinery cannot terminate the caller.
update_localnet_tools_bin() {
  local _WORKDIR="${1:-$WORKDIR}"
  [ "$_WORKDIR" = "localnet" ] || return 0
  [ "${CFG_walrus_enabled:-false}" = "true" ] || return 0

  # Already present (precompiled from a prior run, or a dev build)? Nothing to do.
  if update_WALRUS_LOCALNET_SETUP_BIN_var; then
    return 0
  fi

  if type -t init_app_obj >/dev/null 2>&1 && type -t app_call >/dev/null 2>&1; then
    # Reached only when the binary is absent, i.e. the first 'start'/'regen' after
    # enabling walrus: on dev this source-builds the heavy ~827-crate
    # '--features localnet' graph; on main/staging it fetches the precompiled
    # asset. Once staged in workdirs/common/bin, the early return above skips this
    # on every later start/regen -- so it is genuinely done once. The build is
    # otherwise silent (app_call output is suppressed here and cargo logs to a
    # file), so announce it or it looks like a hang.
    echo "Building localnet tools (done once, might take a long time...)"
    [ -n "$SUIBASE_LOGS_DIR" ] && echo "  (full log: $SUIBASE_LOGS_DIR/cargo-build.log)"
    # Let the build progress reach the terminal (the generic rust builder streams
    # cargo's "Compiling ..." to terminal + log) -- suppress only the app-object
    # setup chatter. Non-fatal: a still-missing binary is handled by the check
    # below and by deploy_walrus_localnet()'s warn-and-skip.
    (
      init_app_obj "localnet_tools" "$_WORKDIR" >/dev/null 2>&1 &&
        app_call "localnet_tools" "install"
    ) || true
    if update_WALRUS_LOCALNET_SETUP_BIN_var; then
      echo "localnet tools ready."
    fi
  fi
  return 0
}
export -f update_localnet_tools_bin

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

  # Ensure the precompiled setup binary is present (fetched from
  # chainmovers/sui-binaries on localnet; a dev build of rust/walrus-store also
  # works). Non-fatal: a missing binary just means "no localnet Walrus this run".
  update_localnet_tools_bin "$_WORKDIR"
  if ! update_WALRUS_LOCALNET_SETUP_BIN_var; then
    warn_user "walrus-localnet-deploy binary not found; skipping localnet Walrus deploy."
    return 0
  fi

  # The localnet Sui must be up (we publish to it).
  if ! wait_for_localnet_rpc "$_RPC"; then
    warn_user "localnet Sui RPC ($_RPC) not reachable; skipping localnet Walrus deploy."
    return 0
  fi

  # Idempotency: skip when the descriptor already matches the live chain id, so a
  # plain 'start' over an existing deployment is a ~0s no-op. Only 'regen' (which
  # wipes the chain and changes its id) triggers a redeploy.
  local _LIVE_CHAIN_ID
  _LIVE_CHAIN_ID=$(curl -fsS -m 5 -X POST "$_RPC" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"sui_getChainIdentifier","params":[]}' 2>/dev/null |
    sed 's/.*"result":"//;s/".*//')
  if [ -n "$_LIVE_CHAIN_ID" ] && [ -f "$_DESCRIPTOR" ] && [ -f "$_WALRUS_CONFIG" ]; then
    local _PREV_CHAIN_ID
    _PREV_CHAIN_ID=$(sed -n 's/^chain_id:[[:space:]]*//p' "$_DESCRIPTOR" | head -1)
    if [ -n "$_PREV_CHAIN_ID" ] && [ "$_PREV_CHAIN_ID" = "$_LIVE_CHAIN_ID" ]; then
      return 0 # already deployed for this chain
    fi
  fi

  # Deploy: publish + off-node N=1 committee stake + write config/descriptor.
  # Contracts are embedded in the binary (no --contracts path needed).
  echo "Deploying localnet Walrus..."
  mkdir -p "$_CONFIG_DEFAULT"
  if ! "$WALRUS_LOCALNET_SETUP_BIN" deploy \
    --rpc "$_RPC" \
    --faucet "$_FAUCET" \
    --wallet "$WORKDIRS/$_WORKDIR/config/client.yaml" \
    --out-config "$_WALRUS_CONFIG" \
    --out-descriptor "$_DESCRIPTOR" \
    --n-shards 1000 \
    --chain-id "$_LIVE_CHAIN_ID"; then
    warn_user "localnet Walrus deploy failed (non-fatal); WalrusStore localnet will be unavailable until the next successful '$_WORKDIR start'/'regen'."
    return 0
  fi

  return 0
}
export -f deploy_walrus_localnet

# True (returns 0) when walrus_enabled=true on localnet but the Walrus Move
# contracts are NOT deployed for the *current* chain: either the descriptor /
# walrus-config is missing, or the descriptor's chain_id does not match the live
# localnet chain id. This mirrors the idempotency check in deploy_walrus_localnet()
# (a regen wipes the chain and changes its id, which is what (re)deploys the
# contracts), so it is the signal for "a regen is needed".
#
# When the localnet Sui RPC is not reachable (node down), a stale chain id cannot
# be proven, so it only reports "needed" when the descriptor/config is missing
# outright -- this avoids a false alarm for a stopped-but-deployed localnet.
is_walrus_localnet_deploy_needed() {
  local _WORKDIR="${1:-$WORKDIR}"

  # Nodeless localnet Walrus is localnet-only and opt-in (mirrors deploy gating).
  [ "$_WORKDIR" = "localnet" ] || return 1
  [ "${CFG_walrus_enabled:-false}" = "true" ] || return 1

  local _CONFIG_DEFAULT="$WORKDIRS/$_WORKDIR/config-default"
  local _DESCRIPTOR="$_CONFIG_DEFAULT/walrus-localnet.yaml"
  local _WALRUS_CONFIG="$_CONFIG_DEFAULT/walrus-config.yaml"

  # Never deployed (or only partially written) -> a regen is needed.
  if [ ! -f "$_DESCRIPTOR" ] || [ ! -f "$_WALRUS_CONFIG" ]; then
    return 0
  fi

  # Descriptor present: confirm it is for the live chain. If the node is not
  # answering we cannot prove a mismatch, so assume it is fine (deployed).
  local _RPC="http://localhost:9000"
  local _LIVE_CHAIN_ID
  _LIVE_CHAIN_ID=$(curl -fsS -m 3 -X POST "$_RPC" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"sui_getChainIdentifier","params":[]}' 2>/dev/null |
    sed 's/.*"result":"//;s/".*//')
  if [ -z "$_LIVE_CHAIN_ID" ]; then
    return 1
  fi
  local _PREV_CHAIN_ID
  _PREV_CHAIN_ID=$(sed -n 's/^chain_id:[[:space:]]*//p' "$_DESCRIPTOR" | head -1)
  if [ -n "$_PREV_CHAIN_ID" ] && [ "$_PREV_CHAIN_ID" = "$_LIVE_CHAIN_ID" ]; then
    return 1 # deployed for this chain
  fi
  return 0 # missing/stale chain id -> (re)deploy via regen
}
export -f is_walrus_localnet_deploy_needed

# Emit the standard "walrus enabled but contracts not deployed" advisory.
# Non-fatal (warn_user goes to stderr, no exit). Shared by 'localnet start' and
# 'localnet status' so the wording stays identical in both.
warn_walrus_localnet_deploy_needed() {
  local _WORKDIR="${1:-${WORKDIR:-localnet}}"
  warn_user "walrus_enabled is true but the Walrus contracts are not deployed on this localnet. Run '$_WORKDIR regen' to deploy them."
}
export -f warn_walrus_localnet_deploy_needed
