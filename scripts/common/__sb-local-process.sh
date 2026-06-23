# shellcheck shell=bash

# You must source __globals.sh before __sb-local-process.sh
#
# Lifecycle for "sb-local": the standalone, localnet-only HTTP server that exposes the
# Walrus aggregator + publisher wire API (GET/PUT /v1/blobs, quilts), backed by the
# self-contained LocalnetMockStore engine (no storage nodes). It is managed exactly like the localnet faucet:
# started on 'localnet start' and stopped on 'localnet stop', gated on
# walrus_local_enabled=true. The suibase-daemon is NOT involved.
#
# It is a glibc binary shipped in the 'localnet-tools' asset (alongside
# walrus-localnet-deploy); on dev it is source-built from rust/localnet-tools.
# See docs/dev/SB_LOCAL_PLAN.md.

# Resolve the sb-local binary (precompiled in workdirs/common/bin, or a release dev build).
# Always release — never target/debug (walrus-core encoding is ~50x slower unoptimized).
# Mirrors update_WALRUS_LOCALNET_SETUP_BIN_var in __walrus-localnet-deploy.sh.
update_SB_LOCAL_BIN_var() {
  SB_LOCAL_BIN=""
  local _candidates=(
    "$WORKDIRS/common/bin/sb-local"
    "$SUIBASE_DIR/rust/localnet-tools/target/release/sb-local"
  )
  local _c
  for _c in "${_candidates[@]}"; do
    if [ -f "$_c" ]; then
      SB_LOCAL_BIN="$_c"
      return 0
    fi
  done
  return 1
}
export -f update_SB_LOCAL_BIN_var

update_SB_LOCAL_PROCESS_PID_var() {
  # success/failure reflected by SB_LOCAL_PROCESS_PID (unset when not running).
  #
  # Match the running sb-local regardless of WHICH candidate path launched it
  # (precompiled common/bin vs a dev target build). Matching only the currently
  # *resolved* path would miss a process started from a different path — e.g. after a
  # mid-session rebuild/install flips the resolver from target/release to common/bin —
  # so 'stop' would no-op (leaking the old process) and 'start' would try (and fail) to
  # bind a duplicate. Probe every candidate path; first match wins.
  local _candidates=(
    "$WORKDIRS/common/bin/sb-local"
    "$SUIBASE_DIR/rust/localnet-tools/target/release/sb-local"
  )
  local _c _PID
  for _c in "${_candidates[@]}"; do
    _PID=$(get_process_pid "$_c")
    if [ "$_PID" != "NULL" ]; then
      export SB_LOCAL_PROCESS_PID="$_PID"
      return
    fi
  done
  unset SB_LOCAL_PROCESS_PID
}
export -f update_SB_LOCAL_PROCESS_PID_var

# True (0) when sb-local should run: localnet workdir + walrus_local_enabled=true.
is_sb_local_supported() {
  local _WORKDIR="${1:-$WORKDIR}"
  [ "$_WORKDIR" = "localnet" ] || return 1
  [ "${CFG_walrus_local_enabled:-false}" = "true" ] || return 1
  return 0
}
export -f is_sb_local_supported

# Bind/port helpers (sb-local has its OWN, independent settings; defaults are used if
# the workdir suibase.yaml does not override them).
sb_local_walrus_port() { echo "${CFG_sb_local_walrus_port:-45840}"; }
export -f sb_local_walrus_port
sb_local_host_ip() { echo "${CFG_sb_local_host_ip:-localhost}"; }
export -f sb_local_host_ip

# Start sb-local (noop if unsupported, already running, not yet deployed, or no binary).
# NON-FATAL on failure: the localnet + the Rust WalrusLocalClient SDK still work without
# the HTTP facade, so a problem here only warns (unlike the faucet, which is required).
start_sb_local_process() {
  is_sb_local_supported || return 0

  # sb-local connects to the deployed Walrus system, so the deploy descriptor must
  # exist. If not (walrus just enabled, no regen yet), skip silently — the
  # "regen recommended" advisory is emitted elsewhere.
  local _DESCRIPTOR="$WORKDIRS/localnet/config/walrus-localnet.yaml"
  if [ ! -f "$_DESCRIPTOR" ]; then
    return 0
  fi

  # Skip if the on-chain deploy is STALE (descriptor chain id != live chain id, e.g.
  # after a chain wipe): its system/exchange objects no longer exist, so opening the
  # store would fail. The "run 'localnet regen'" advisory is surfaced by the workdir-exec
  # status/start path; here we just avoid a noisy connect error against a dead deploy.
  if type -t is_walrus_localnet_deploy_needed >/dev/null 2>&1 &&
    is_walrus_localnet_deploy_needed localnet; then
    return 0
  fi

  if ! update_SB_LOCAL_BIN_var; then
    warn_user "Walrus API server binary not found; the Walrus localnet HTTP API will be unavailable."
    return 0
  fi

  update_SB_LOCAL_PROCESS_PID_var
  if [ -n "$SB_LOCAL_PROCESS_PID" ]; then
    return 0
  fi

  local _PORT _BIND _DIR
  _PORT="$(sb_local_walrus_port)"
  _BIND="$(sb_local_host_ip)"
  _DIR="$WORKDIRS/localnet/sb-local"
  mkdir -p "$_DIR"

  echo "Starting Walrus API on http://$_BIND:$_PORT"
  rm -f "$_DIR/sb-local.log" >/dev/null 2>&1

  "$SB_LOCAL_BIN" --bind "$_BIND" --port "$_PORT" --workdir localnet \
    >"$_DIR/sb-local.log" 2>&1 &

  # Wait until /status answers, or a hard failure shows in the log, or timeout. The window
  # must EQUAL-OR-EXCEED sb-local's own connect-retry budget (CONNECT_MAX_ATTEMPTS in
  # walrus-local-sdk, ~80s) so we observe its real outcome -- success (/status) or its own
  # differentiated error -- instead of giving up while it is still polling the warming-up node.
  local end=$((SECONDS + 90))
  local ALIVE=false
  local AT_LEAST_ONE_SECOND=false
  while [ $SECONDS -lt $end ]; do
    if curl -x "" -s -m 2 "http://$_BIND:$_PORT/status" 2>/dev/null | grep -q "OK"; then
      ALIVE=true
      break
    fi
    if [ -f "$_DIR/sb-local.log" ] &&
      grep -qi "address already in use\|panicked\|^Error:" "$_DIR/sb-local.log"; then
      break
    fi
    echo -n "."
    sleep 1
    AT_LEAST_ONE_SECOND=true
  done
  [ "$AT_LEAST_ONE_SECOND" = true ] && echo

  if [ "$ALIVE" = false ]; then
    # Differentiate the cause so the suggested next action is correct (all branches non-fatal).
    if type -t is_walrus_localnet_deploy_needed >/dev/null 2>&1 &&
      is_walrus_localnet_deploy_needed localnet; then
      # Descriptor missing, or its chain id != the live chain id: the deploy is for another
      # chain. Only a regen changes the chain id, so this genuinely needs a redeploy.
      warn_user "Walrus API not started: contracts not deployed for this localnet chain -> run 'localnet regen' (walrus_local_enabled=true)."
    elif type -t wait_for_localnet_rpc >/dev/null 2>&1 &&
      ! wait_for_localnet_rpc "http://localhost:9000" 1; then
      # The node itself is not answering JSON-RPC.
      warn_user "Walrus API not started: localnet node not responding -> re-run 'localnet start'."
    else
      # Node up and the deploy matches this chain, but its gRPC has not served the Walrus
      # system object within the budget: still warming up (common on a slow cold start).
      # A plain restart -- not a regen -- is the fix.
      warn_user "Walrus API not started: localnet still warming up (gRPC not serving objects yet) -> re-run 'localnet start' (regen only if it persists)."
    fi
    [ -f "$_DIR/sb-local.log" ] && tail -4 "$_DIR/sb-local.log"
    return 0
  fi

  update_SB_LOCAL_PROCESS_PID_var
  echo "Walrus API started ( pid $SB_LOCAL_PROCESS_PID )"
}
export -f start_sb_local_process

# Build the localnet-tools binaries as-needed, then start sb-local. The REBUILD half is the
# daemon's logic (is_installed-driven -- see update_localnet_tools_bin and
# start_suibase_daemon_as_needed): a 'localnet start' after '~/suibase/update' self-heals a
# stale/missing binary before starting it. The LIFECYCLE half is NOT the daemon's: unlike the
# always-on suibase-daemon, sb-local is a localnet-scoped service -- started here on
# 'localnet start' and stopped by stop_all_services on 'localnet stop'. So there is no
# always-on hot restart-on-upgrade; a running sb-local picks up a new binary on the next
# localnet stop/start (or regen), its natural lifecycle event. NON-FATAL (the localnet + the
# Rust WalrusLocalClient SDK work without the HTTP facade); gated on
# localnet + walrus_local_enabled via is_sb_local_supported.
start_sb_local_process_as_needed() {
  is_sb_local_supported || return 0

  # Build/install as-needed (no-op once is_installed). Analog of the daemon's `app_call install`.
  if type -t update_localnet_tools_bin >/dev/null 2>&1; then
    update_localnet_tools_bin "$WORKDIR"
  fi

  # Start as-needed: no-op if already running, or if the deploy descriptor is missing/stale,
  # or the binary is still absent.
  start_sb_local_process
}
export -f start_sb_local_process_as_needed

# Stop sb-local. ps-reap-safe: like stop_walrus_relay_process, poll the SAME `ps`
# view callers use until it clears (kernel-reap lag can briefly still list a
# just-SIGKILL'd process on slow CI). Noop if not running.
stop_sb_local_process() {
  update_SB_LOCAL_PROCESS_PID_var
  if [ -z "$SB_LOCAL_PROCESS_PID" ]; then
    return 0
  fi

  echo "Stopping Walrus API ( pid $SB_LOCAL_PROCESS_PID )"
  kill "$SB_LOCAL_PROCESS_PID" 2>/dev/null || true

  local count=0
  while [ $count -lt 10 ] && kill -0 "$SB_LOCAL_PROCESS_PID" 2>/dev/null; do
    sleep 1
    count=$((count + 1))
  done
  if kill -0 "$SB_LOCAL_PROCESS_PID" 2>/dev/null; then
    echo "Force killing Walrus API process"
    kill -9 "$SB_LOCAL_PROCESS_PID" 2>/dev/null || true
  fi

  # Settle the `ps` view before returning.
  local _settle=0
  update_SB_LOCAL_PROCESS_PID_var
  while [ -n "$SB_LOCAL_PROCESS_PID" ] && [ "$_settle" -lt 5 ]; do
    kill -9 "$SB_LOCAL_PROCESS_PID" 2>/dev/null || true
    sleep 1
    _settle=$((_settle + 1))
    update_SB_LOCAL_PROCESS_PID_var
  done
}
export -f stop_sb_local_process
