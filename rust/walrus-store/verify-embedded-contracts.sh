#!/bin/bash
# Drift-guard for the vendored Walrus Move contracts embedded into
# walrus-localnet-deploy (include_dir! of embedded-contracts/).
#
# Two checks:
#   1. Integrity: the committed embedded-contracts/CONTRACTS.sha256 still matches
#      the vendored files (no local tampering / partial edits).
#   2. Upstream parity (when a walrus checkout is available): the vendored dirs are
#      byte-identical to contracts/{wal,wal_exchange,walrus,walrus_subsidies} at the
#      walrus rev pinned in Cargo.toml. Set WALRUS_DIR to a checkout, or it clones
#      the pinned rev over HTTPS (avoids the WSL2 SSH->GitHub hang).
#
# Run locally or in CI. Exit non-zero on drift. Bump the crate version + re-vendor
# whenever the pinned walrus rev changes.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EMB="$HERE/embedded-contracts"
MANIFEST="$EMB/CONTRACTS.sha256"
PKGS=(wal wal_exchange walrus walrus_subsidies)

# --- 1. Integrity: committed manifest still matches the vendored files ---
if [ ! -f "$MANIFEST" ]; then
  echo "ERROR: $MANIFEST missing" >&2
  exit 1
fi
( cd "$EMB" && sha256sum -c --quiet CONTRACTS.sha256 ) || {
  echo "ERROR: embedded-contracts/ does not match CONTRACTS.sha256 (drift or tamper)." >&2
  echo "       Re-vendor from the pinned rev and regenerate the manifest." >&2
  exit 1
}
echo "OK: embedded-contracts integrity (CONTRACTS.sha256) verified."

# --- 2. Upstream parity against the pinned walrus rev ---
REV=$(grep -m1 'rev = "' "$HERE/Cargo.toml" | sed 's/.*rev = "\([0-9a-f]*\)".*/\1/')
if [ -z "$REV" ]; then
  echo "WARN: could not parse the pinned walrus rev from Cargo.toml; skipping upstream parity." >&2
  exit 0
fi

CLEANUP=""
if [ -n "${WALRUS_DIR:-}" ] && [ -d "$WALRUS_DIR/contracts" ]; then
  SRC="$WALRUS_DIR/contracts"
else
  TMP=$(mktemp -d)
  CLEANUP="$TMP"
  echo "Cloning MystenLabs/walrus @ $REV (shallow, HTTPS) for parity check..."
  git -C "$TMP" init -q
  git -C "$TMP" remote add origin https://github.com/MystenLabs/walrus.git
  if ! git -C "$TMP" fetch -q --depth 1 origin "$REV" 2>/dev/null; then
    echo "WARN: could not fetch walrus rev $REV (offline?); skipping upstream parity." >&2
    [ -n "$CLEANUP" ] && rm -rf "$CLEANUP"
    exit 0
  fi
  git -C "$TMP" checkout -q FETCH_HEAD
  SRC="$TMP/contracts"
fi

drift=0
for p in "${PKGS[@]}"; do
  if ! diff -rq "$SRC/$p" "$EMB/$p" >/dev/null 2>&1; then
    echo "DRIFT: embedded-contracts/$p differs from upstream contracts/$p @ $REV" >&2
    drift=1
  fi
done
[ -n "$CLEANUP" ] && rm -rf "$CLEANUP"

if [ "$drift" -ne 0 ]; then
  echo "ERROR: vendored contracts diverge from the pinned walrus rev $REV." >&2
  echo "       Re-vendor (cp -a) + regenerate CONTRACTS.sha256 + bump the crate version." >&2
  exit 1
fi
echo "OK: embedded-contracts match upstream walrus @ $REV."
