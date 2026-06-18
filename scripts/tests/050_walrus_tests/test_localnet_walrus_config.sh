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
check "walrus_enabled: false default in defaults/localnet/suibase.yaml" \
    grep -qE "^walrus_enabled:[[:space:]]*false" "$SCRIPTS/defaults/localnet/suibase.yaml"

# 3. consts.yaml precompiled-binary app registration.
CONSTS="$SCRIPTS/defaults/consts.yaml"
check "consts.yaml: 16 localnet_tools_* keys" \
    bash -c "[ \$(grep -cE '^localnet_tools_' '$CONSTS') -eq 16 ]"
check "consts.yaml: install_type=user (-> workdirs/common/bin)" \
    grep -qE '^localnet_tools_install_type:[[:space:]]*"user"' "$CONSTS"
check "consts.yaml: support_version_check=false (bin has no --version)" \
    grep -qE '^localnet_tools_support_version_check:[[:space:]]*false' "$CONSTS"
check "consts.yaml: force_tag scopes the user-install fetch (localnet-tools asset)" \
    grep -qE '^localnet_tools_force_tag:[[:space:]]*"localnet-tools-v' "$CONSTS"
# Daemon-consistent: a suibase-built rust asset -> source-build on dev branches,
# precompiled on main/staging (see sb_app_rust_build_and_install in __apps.sh).
check "consts.yaml: src_type=suibase (routes through the app build system)" \
    grep -qE '^localnet_tools_src_type:[[:space:]]*"suibase"' "$CONSTS"
check "consts.yaml: build_type=rust" \
    grep -qE '^localnet_tools_build_type:[[:space:]]*"rust"' "$CONSTS"
check "consts.yaml: src_path=rust/walrus-store" \
    grep -qE '^localnet_tools_src_path:[[:space:]]*"rust/walrus-store"' "$CONSTS"

# 4. Deploy script: syntax + functions + gate + idempotency.
DEPLOY="$COMMON/__walrus-localnet-deploy.sh"
check "__walrus-localnet-deploy.sh exists" test -f "$DEPLOY"
check "__walrus-localnet-deploy.sh passes bash -n" bash -n "$DEPLOY"
check "defines deploy_walrus_localnet()" grep -qE "^deploy_walrus_localnet\(\)" "$DEPLOY"
check "defines update_localnet_tools_bin()" grep -qE "^update_localnet_tools_bin\(\)" "$DEPLOY"
check "defines update_WALRUS_LOCALNET_SETUP_BIN_var()" grep -qE "^update_WALRUS_LOCALNET_SETUP_BIN_var\(\)" "$DEPLOY"
check "deploy gated on CFG_walrus_enabled" grep -qE 'CFG_walrus_enabled.*!=.*"true"' "$DEPLOY"
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

echo
if [ "$_fail" -eq 0 ]; then
    echo "PASS: M1 localnet Walrus wiring"
    exit 0
else
    echo "FAIL: M1 localnet Walrus wiring" >&2
    exit 1
fi
