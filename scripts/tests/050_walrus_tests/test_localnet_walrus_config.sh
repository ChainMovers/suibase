#!/bin/bash

# Non-destructive checks for the nodeless localnet Walrus (M1) wiring.
#
# Verifies the opt-in config gate, the localnet walrus-config template, the
# precompiled-binary app registration, and the deploy/regen wiring are present and
# correct -- WITHOUT starting/regenerating localnet, so it is safe + fast for the
# scripts-tests CI (no side effects on ~/suibase/workdirs).
#
# The actual enabled deploy + store/read/extend round-trip is exercised separately
# (it needs the precompiled binary + a destructive regen) and is not part of this
# fast static test. See docs/dev/LOCALNET_WALRUS_PLAN.md.

# Ignore SIGPIPE on macOS (consistent with the other 050 tests).
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
fi

# This test takes no options; run-all.sh passes --skip_init (+ passthru) -- ignored.

SUIBASE_DIR="$HOME/suibase"
SCRIPTS="$SUIBASE_DIR/scripts"
COMMON="$SCRIPTS/common"

_fail=0
check() {
    # check "description" <command...>  -- command must succeed for a pass.
    local _desc="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        echo "  ok  : $_desc"
    else
        echo "  FAIL: $_desc" >&2
        _fail=1
    fi
}

echo "=== M1: nodeless localnet Walrus wiring (non-destructive) ==="

# 1. localnet walrus-config.yaml template (deploy-filled placeholders).
TEMPLATE="$SCRIPTS/templates/localnet/config-default/walrus-config.yaml"
check "localnet walrus-config.yaml template exists" test -f "$TEMPLATE"
# grep-based (NO PyYAML dependency — the rust-tests CI runner has no pyyaml installed;
# the deploy bin validates the REAL written config via serde_yaml at deploy time).
check "template default_context: localnet" \
    grep -qE '^default_context:[[:space:]]*localnet([[:space:]]|$)' "$TEMPLATE"
check "template declares a contexts: block" grep -qE '^contexts:' "$TEMPLATE"
check "template has a localnet: context entry" grep -qE '^[[:space:]]+localnet:' "$TEMPLATE"
check "template localnet rpc -> http://localhost:9000" \
    grep -qE 'http://localhost:9000' "$TEMPLATE"

# 2. CRITICAL: opt-in, disabled by default (default localnet start must be unchanged).
check "walrus_local_enabled: false default in defaults/localnet/suibase.yaml" \
    grep -qE "^walrus_local_enabled:[[:space:]]*false" "$SCRIPTS/defaults/localnet/suibase.yaml"

# 3. consts.yaml precompiled-binary app registration.
CONSTS="$SCRIPTS/defaults/consts.yaml"
check "consts.yaml: 16 localnet_tools_* keys" \
    bash -c "[ \$(grep -cE '^localnet_tools_' '$CONSTS') -eq 16 ]"
check "consts.yaml: install_type=user (-> workdirs/common/bin)" \
    grep -qE '^localnet_tools_install_type:[[:space:]]*"user"' "$CONSTS"
check "consts.yaml: support_version_check=false (bin has no --version)" \
    grep -qE '^localnet_tools_support_version_check:[[:space:]]*false' "$CONSTS"
check "consts.yaml: force_tag unpinned (~) — auto-latest, no per-release edit" \
    grep -qE '^localnet_tools_force_tag:[[:space:]]*~' "$CONSTS"
check "consts.yaml: asset_name_filter scopes user-install to the localnet-tools family" \
    grep -qE '^localnet_tools_asset_name_filter:[[:space:]]*"localnet-tools"' "$CONSTS"
# Daemon-consistent: a suibase-built rust asset -> source-build on dev branches,
# precompiled on main/staging (see sb_app_rust_build_and_install in __apps.sh).
check "consts.yaml: src_type=suibase (routes through the app build system)" \
    grep -qE '^localnet_tools_src_type:[[:space:]]*"suibase"' "$CONSTS"
check "consts.yaml: build_type=rust" \
    grep -qE '^localnet_tools_build_type:[[:space:]]*"rust"' "$CONSTS"
check "consts.yaml: src_path=rust/localnet-tools (the bins crate)" \
    grep -qE '^localnet_tools_src_path:[[:space:]]*"rust/localnet-tools"' "$CONSTS"

# 4. Deploy script: syntax + functions + gate + idempotency.
DEPLOY="$COMMON/__walrus-localnet-deploy.sh"
check "__walrus-localnet-deploy.sh exists" test -f "$DEPLOY"
check "__walrus-localnet-deploy.sh passes bash -n" bash -n "$DEPLOY"
check "defines deploy_walrus_localnet()" grep -qE "^deploy_walrus_localnet\(\)" "$DEPLOY"
check "defines update_localnet_tools_bin()" grep -qE "^update_localnet_tools_bin\(\)" "$DEPLOY"
check "defines update_WALRUS_LOCALNET_SETUP_BIN_var()" grep -qE "^update_WALRUS_LOCALNET_SETUP_BIN_var\(\)" "$DEPLOY"
check "deploy gated on CFG_walrus_local_enabled" grep -qE 'CFG_walrus_local_enabled.*!=.*"true"' "$DEPLOY"
check "deploy has chain-id idempotency (skip on match)" grep -q "_PREV_CHAIN_ID" "$DEPLOY"
check "deploy does not pass --contracts as a flag (uses embedded)" \
    bash -c "! grep -qE '^[[:space:]]*--contracts ' '$DEPLOY'"

# 4b. Daemon-consistent build path: the generic per-asset rust builder + the dev
#     force-rebuild command (mirrors scripts/dev/update-daemon).
APPS="$COMMON/__apps.sh"
check "__apps.sh defines sb_app_rust_build_and_install_generic()" \
    grep -qE "^sb_app_rust_build_and_install_generic\(\)" "$APPS"
check "__apps.sh routes non-daemon assets to the generic builder" \
    grep -q "sb_app_rust_build_and_install_generic" "$APPS"
DEVCMD="$SCRIPTS/dev/update-localnet-tools"
check "dev/update-localnet-tools exists + executable" test -x "$DEVCMD"
check "dev/update-localnet-tools passes bash -n" bash -n "$DEVCMD"

# 5. Regen hook in __workdir-exec.sh.
WEXEC="$COMMON/__workdir-exec.sh"
check "__workdir-exec.sh passes bash -n" bash -n "$WEXEC"
check "sources __walrus-localnet-deploy.sh" grep -q "__walrus-localnet-deploy.sh" "$WEXEC"
check "calls deploy_walrus_localnet" grep -q "deploy_walrus_localnet" "$WEXEC"

# 6. The testnet/mainnet relay path is unchanged (localnet not added to it, so
#    enabling localnet Walrus does not trigger relay/site-builder binary installs).
check "is_walrus_supported_by_workdir gate unchanged (testnet/mainnet only)" \
    grep -qE '\[ "\$_workdir" = "testnet" \] \|\| \[ "\$_workdir" = "mainnet" \]' "$COMMON/__globals.sh"

# 7. "regen recommended" advisory: enabled but contracts not deployed surfaces a
#    non-fatal warning on 'localnet start' (fast path) and 'localnet status'.
check "defines is_walrus_localnet_deploy_needed()" \
    grep -qE "^is_walrus_localnet_deploy_needed\(\)" "$DEPLOY"
check "defines warn_walrus_localnet_deploy_needed()" \
    grep -qE "^warn_walrus_localnet_deploy_needed\(\)" "$DEPLOY"
check "deploy-needed predicate gated on CFG_walrus_local_enabled + localnet" \
    bash -c "grep -A8 '^is_walrus_localnet_deploy_needed()' '$DEPLOY' | grep -q 'CFG_walrus_local_enabled'"
check "advisory recommends a regen" grep -qE "regen' to deploy them" "$DEPLOY"
check "status/start call the deploy-needed predicate" \
    grep -q "is_walrus_localnet_deploy_needed" "$WEXEC"
check "status/start emit the advisory helper" \
    grep -q "warn_walrus_localnet_deploy_needed" "$WEXEC"
check "advisory call sites guarded on WORKDIR=localnet" \
    bash -c "[ \$(grep -c 'is_walrus_localnet_deploy_needed \"\$WORKDIR\"' '$WEXEC') -eq 2 ]"

# 7b. Functional: pure (no live localnet) branches of the predicate. The deploy
#     script is all function defs, so it can be sourced standalone. ($1 = DEPLOY.)
check "predicate: walrus disabled -> not needed" \
    bash -c 'source "$1"; WORKDIRS="$(mktemp -d)"; mkdir -p "$WORKDIRS/localnet/config-default";
             CFG_walrus_local_enabled=false; ! is_walrus_localnet_deploy_needed localnet' _ "$DEPLOY"
check "predicate: non-localnet workdir -> not needed" \
    bash -c 'source "$1"; WORKDIRS="$(mktemp -d)";
             CFG_walrus_local_enabled=true; ! is_walrus_localnet_deploy_needed testnet' _ "$DEPLOY"
check "predicate: enabled + descriptor missing -> needed" \
    bash -c 'source "$1"; WORKDIRS="$(mktemp -d)"; mkdir -p "$WORKDIRS/localnet/config-default";
             CFG_walrus_local_enabled=true; is_walrus_localnet_deploy_needed localnet' _ "$DEPLOY"
check "warn helper recommends '<workdir> regen'" \
    bash -c 'source "$1"; warn_user() { printf "%s\n" "$*"; };
             warn_walrus_localnet_deploy_needed localnet 2>&1 | grep -q "localnet regen"' _ "$DEPLOY"

# 8. sb-local wiring (the localnet Walrus aggregator/publisher HTTP server).
SBP="$COMMON/__sb-local-process.sh"
check "__sb-local-process.sh exists" test -f "$SBP"
check "__sb-local-process.sh passes bash -n" bash -n "$SBP"
check "defines start_sb_local_process()" grep -qE "^start_sb_local_process\(\)" "$SBP"
check "defines stop_sb_local_process()" grep -qE "^stop_sb_local_process\(\)" "$SBP"
check "defines update_SB_LOCAL_PROCESS_PID_var()" grep -qE "^update_SB_LOCAL_PROCESS_PID_var\(\)" "$SBP"
check "start gated on localnet + walrus_local_enabled (is_sb_local_supported)" \
    grep -q "is_sb_local_supported" "$SBP"
check "sb-local stop is ps-reap-safe (settle loop)" grep -q "_settle" "$SBP"

# Own, independent bind/port settings (default localhost / 45840 — Walrus 458xx range).
LN_YAML="$SCRIPTS/defaults/localnet/suibase.yaml"
check "localnet defaults define sb_local_walrus_port (45840, Walrus 458xx localnet slot)" \
    grep -qE "^sb_local_walrus_port:[[:space:]]*45840" "$LN_YAML"
check "localnet defaults define sb_local_host_ip" grep -qE "^sb_local_host_ip:" "$LN_YAML"

# Lifecycle wiring: start/stop_all_services + status + post-deploy start.
GLOB="$COMMON/__globals.sh"
check "start_all_services starts sb-local" grep -q "start_sb_local_process" "$GLOB"
check "stop_all_services stops sb-local" grep -q "stop_sb_local_process" "$GLOB"
check "workdir-exec sources __sb-local-process.sh" grep -q "__sb-local-process.sh" "$WEXEC"
check "workdir-exec starts sb-local after deploy" \
    bash -c "grep -A4 'deploy_walrus_localnet \"\$WORKDIR\"' '$WEXEC' | grep -q 'start_sb_local_process'"
check "status shows the 'Walrus API' line" grep -q '"Walrus API"' "$WEXEC"

# Producer/consumer: bin_names carries BOTH binaries; the dev builder is multi-bin.
check "consts.yaml: localnet_tools_bin_names includes sb-local" \
    grep -qE '^localnet_tools_bin_names:.*sb-local' "$CONSTS"
check "consts.yaml: localnet_tools_bin_names still includes walrus-localnet-deploy" \
    grep -qE '^localnet_tools_bin_names:.*walrus-localnet-deploy' "$CONSTS"
check "generic rust builder iterates bin_names (no hardcoded single --bin)" \
    bash -c "! grep -qE 'CARGO_ARGS=\(--release --features localnet --bin walrus-localnet-deploy\)' '$APPS'"
# The bins crate enables walrus-store's localnet feature via its dependency, so the
# builder must NOT pass --features for localnet-tools.
check "generic rust builder uses --release (no --features) for localnet-tools" \
    bash -c "grep -A8 'localnet-tools)' '$APPS' | grep -qE '_CARGO_ARGS=\(--release\)'"

# Crate layout: the BINS live in rust/localnet-tools; walrus-store is a thin lib (no bins).
BINS_CARGO="$SUIBASE_DIR/rust/localnet-tools/Cargo.toml"
LIB_CARGO="$SUIBASE_DIR/rust/walrus-store/Cargo.toml"
check "rust/localnet-tools/Cargo.toml exists (bins crate)" test -f "$BINS_CARGO"
check "localnet-tools declares the sb-local [[bin]]" grep -qE '^name = "sb-local"' "$BINS_CARGO"
check "localnet-tools declares the walrus-localnet-deploy [[bin]]" \
    grep -qE '^name = "walrus-localnet-deploy"' "$BINS_CARGO"
check "localnet-tools depends on walrus-store with the localnet feature" \
    bash -c "grep -q 'walrus-store = {.*features = \\[\"localnet\"\\]' '$BINS_CARGO'"
check "walrus-store is lib-only (no [[bin]] sections)" \
    bash -c "! grep -qE '^\\[\\[bin\\]\\]' '$LIB_CARGO'"
check "walrus-store no longer pulls bin-only deps (axum/clap) in its localnet feature" \
    bash -c "! grep -qE 'dep:axum|dep:clap' '$LIB_CARGO'"

# 9. The live HTTP round-trip test exists (destructive; self-skips when sb-local is down).
SBHTTP="$SCRIPTS/tests/050_walrus_tests/test_sb_local_http.sh"
check "test_sb_local_http.sh exists" test -f "$SBHTTP"
check "test_sb_local_http.sh passes bash -n" bash -n "$SBHTTP"

echo
if [ "$_fail" -eq 0 ]; then
    echo "PASS: localnet Walrus + sb-local wiring"
    exit 0
else
    echo "FAIL: localnet Walrus + sb-local wiring" >&2
    exit 1
fi
