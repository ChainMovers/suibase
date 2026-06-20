#!/bin/bash

# DESTRUCTIVE: verifies that `localnet regen` properly WIPES the nodeless-Walrus state and
# redeploys cleanly — i.e. there is NO disk accumulation of blob data across regens.
#
# It seeds blob data (via the sb-local publisher), confirms the on-disk runtime dir is
# non-empty, runs `localnet regen`, then asserts: the Walrus contracts were redeployed onto
# a FRESH chain (descriptor present, chain_id changed) AND the blob runtime dir is empty
# again (no leftover bytes/sidecars).
#
# Gated on Walrus being deployed: if the descriptor is absent (e.g. the non-Walrus
# scripts-tests suite, where walrus_local_enabled is off), it SKIPS cleanly (exit 0). It
# regens the localnet, so it belongs in the heavy integration job, not the per-push suite.

set -uo pipefail

SUIBASE_DIR="$HOME/suibase"
LOCALNET_CONFIG="$SUIBASE_DIR/workdirs/localnet/config"
BLOB_DIR="$LOCALNET_CONFIG/walrus-localnet-blobs"
DESCRIPTOR="$LOCALNET_CONFIG/walrus-localnet.yaml"
SB_LOCAL_BASE="${SB_LOCAL_BASE:-http://localhost:45840}"

_fail=0
ok() { echo "  ok  : $*"; }
bad() {
  echo "  FAIL: $*" >&2
  _fail=1
}

chain_id_of() { sed -n 's/^chain_id:[[:space:]]*//p' "$1" 2>/dev/null | head -1; }
blob_count() { ls "$BLOB_DIR" 2>/dev/null | grep -cE '\.(bin|meta)$'; }

if [ ! -f "$DESCRIPTOR" ]; then
  echo "SKIP: nodeless Walrus is not deployed on localnet (no $DESCRIPTOR) — enable"
  echo "      walrus_local_enabled + regen first. (Safe skip in the non-Walrus suite.)"
  exit 0
fi

echo "== seed blob data so the runtime dir is non-empty =="
if curl -x "" -s -m 3 "$SB_LOCAL_BASE/status" 2>/dev/null | grep -q OK; then
  for i in 1 2 3; do
    curl -x "" -s -m 30 -X PUT --data-binary "regen-wipe-seed-$i-$$-$(date +%s 2>/dev/null || echo x)" \
      "$SB_LOCAL_BASE/v1/blobs?epochs=2" >/dev/null 2>&1
  done
else
  echo "  note: sb-local not reachable at $SB_LOCAL_BASE; cannot seed via HTTP"
fi

BEFORE="$(blob_count)"
OLD_CHAIN="$(chain_id_of "$DESCRIPTOR")"
echo "  before regen: blob files=$BEFORE chain_id=$OLD_CHAIN"
if [ "${BEFORE:-0}" -ge 1 ]; then
  ok "runtime had blob data before regen ($BEFORE files)"
else
  # Without seeded data we cannot prove the wipe meaningfully — fail loudly in CI (sb-local
  # is expected up there) rather than pass a vacuous check.
  bad "could not seed blob data before regen (sb-local down or store failing?) — cannot verify wipe"
fi

echo "== localnet regen (wipes chain + redeploys Walrus) =="
if ! "$SUIBASE_DIR/scripts/localnet" regen >/dev/null 2>&1; then
  bad "localnet regen failed"
  echo "FAIL: localnet Walrus regen wipe" >&2
  exit 1
fi

NEW_CHAIN="$(chain_id_of "$DESCRIPTOR")"
AFTER="$(blob_count)"
echo "  after regen:  blob files=$AFTER chain_id=$NEW_CHAIN"

[ -f "$DESCRIPTOR" ] \
  && ok "Walrus redeployed (descriptor present after regen)" \
  || bad "descriptor missing after regen (Walrus not redeployed)"

if [ -n "$NEW_CHAIN" ] && [ "$NEW_CHAIN" != "$OLD_CHAIN" ]; then
  ok "chain wiped + redeployed ($OLD_CHAIN -> $NEW_CHAIN)"
else
  bad "chain_id did not change across regen ($OLD_CHAIN -> $NEW_CHAIN)"
fi

if [ "${AFTER:-1}" -eq 0 ]; then
  ok "blob runtime wiped — NO accumulation (0 .bin/.meta files after regen)"
else
  bad "blob runtime NOT wiped: $AFTER .bin/.meta files remain after regen (accumulation)"
fi

echo
if [ "$_fail" -eq 0 ]; then
  echo "PASS: localnet Walrus regen wipe (clean redeploy, no accumulation)"
  exit 0
else
  echo "FAIL: localnet Walrus regen wipe" >&2
  exit 1
fi
