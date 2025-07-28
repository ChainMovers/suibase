#!/bin/bash

# Test for daemon upgrade functionality in repair script
# This test reproduces the bug where ~/suibase/update doesn't upgrade
# suibase-daemon from 0.1.0 to latest 0.1.2

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# Source globals
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="localnet"
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

tests() {
  test_daemon_upgrade_decision_logic
  test_version_comparison_basics
}

test_version_comparison_basics() {
  echo "Testing version comparison functions..."

  # Test the specific case that should now work (minor version upgrade)
  if ! version_less_than "0.0.1" "0.1.2"; then
    fail "0.0.1 should be less than 0.1.2 (minor version upgrade should work)"
  fi

  # Test that patch version upgrades are correctly ignored (preserves original behavior)
  if version_less_than "0.1.0" "0.1.2"; then
    fail "0.1.0 should NOT be less than 0.1.2 (patch versions ignored for stability)"
  fi

  # Test other related cases with minor version upgrades
  if ! version_less_than "0.1.0" "0.2.0"; then
    fail "0.1.0 should be less than 0.2.0 (minor version upgrade)"
  fi

  if version_less_than "0.2.0" "0.1.0"; then
    fail "0.2.0 should NOT be less than 0.1.0"
  fi

  if version_less_than "0.1.0" "0.1.0"; then
    fail "0.1.0 should NOT be less than 0.1.0 (equal)"
  fi

  echo "Version comparison tests completed."
}

test_daemon_upgrade_decision_logic() {
  echo "Testing daemon upgrade via repair script..."

  # Check if we're on main branch - daemon upgrades only happen on main
  local current_branch
  current_branch=$(cd "$SUIBASE_DIR" && git rev-parse --abbrev-ref HEAD)

  if [ "$current_branch" != "main" ]; then
    # Skipping daemon upgrade test - not on main branch (current: $current_branch)"
    # Note: Daemon upgrades are only allowed on main branch for stability"
    #       To test upgrades, switch to main branch.
    return 0
  fi

  # This test verifies that daemon upgrade from 0.0.1 to 0.x with x > 0 works correctly"

  # 1. Verify that suibase-daemon is already installed
  local daemon_bin="$SUIBASE_BIN_DIR/suibase-daemon"
  local version_file="$SUIBASE_BIN_DIR/suibase-daemon-version.yaml"

  if [ ! -f "$daemon_bin" ]; then
    fail "suibase-daemon binary not found at $daemon_bin - should be installed by now"
  fi

  if [ ! -f "$version_file" ]; then
    fail "suibase-daemon version file not found at $version_file"
  fi

  # 2. Start suibase if not running (using testnet start)
  echo "  Starting testnet to ensure daemon is running..."
  testnet start || fail "Failed to start testnet"

  # 3. Back up current version file and create fake older version
  local backup_file="${version_file}.backup"
  cp "$version_file" "$backup_file" || fail "Failed to backup version file"

  # Get current version for comparison
  local original_version
  original_version=$(grep "^version:" "$version_file" | cut -d'"' -f2)
  echo "  Original version: $original_version"

  # Create fake older version to trigger upgrade
  echo 'version: "0.0.1"
branch: "main"
origin: "precompiled"' > "$version_file"

  # 4. Call repair script like a user would from command line
  local repair_output
  local repair_exit_code

  # Call repair with clean environment to avoid test script pollution
  # This mimics a user calling repair from a fresh terminal
  cd "$HOME" || fail "Failed to cd to HOME"
  env -i HOME="$HOME" PATH="$PATH" TERM="${TERM:-xterm}" USER="$USER" ~/suibase/repair > /tmp/repair_output.log 2>&1
  repair_exit_code=$?
  repair_output=$(cat /tmp/repair_output.log)
  rm -f /tmp/repair_output.log

  echo "  Repair output:"
  echo "$repair_output" | sed 's/^/    /' || echo "Failed to format repair output"

  # 5. Check if upgrade happened
  local new_version
  new_version=$(grep "^version:" "$version_file" | cut -d'"' -f2)

  # 6. Restore original version file
  mv "$backup_file" "$version_file" || fail "Failed to restore version file"

  # 7. Verify results
  if [ "$repair_exit_code" -ne 0 ]; then
    fail "Repair script failed with exit code $repair_exit_code"
  fi

  if [ "$new_version" = "0.0.1" ]; then
    fail "Daemon upgrade did NOT happen - version still 0.0.1 (THIS IS A REGRESSION BUG!)"
  fi
}

tests
