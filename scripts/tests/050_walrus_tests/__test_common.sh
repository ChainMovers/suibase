#!/bin/bash

# Common code for walrus relay test scripts.
# Tests for walrus-upload-relay binary management and process lifecycle

SUIBASE_DIR="$HOME/suibase"
WORKDIRS="$SUIBASE_DIR/workdirs"
WORKDIR="${CI_WORKDIR:-testnet}"  # Use CI_WORKDIR if set, default to testnet

# Validate that WORKDIR is supported for walrus tests
validate_workdir_support() {
    case "$WORKDIR" in
        "testnet"|"mainnet")
            # Supported workdirs
            return 0
            ;;
        *)
            echo "SKIP: Walrus relay tests only support testnet and mainnet workdirs, got: $WORKDIR"
            exit 2
            ;;
    esac
}

# Restore suibase.yaml from default template if different
restore_suibase_yaml_from_default() {
    local workdir="$1"
    local workdir_config="$WORKDIRS/$workdir/suibase.yaml"
    local default_config="$SUIBASE_DIR/scripts/defaults/$workdir/suibase.yaml"
    
    if [ ! -f "$default_config" ]; then
        echo "ERROR: Default template not found at $default_config"
        return 1
    fi
    
    # Check if workdir config is different from default template
    if [ ! -f "$workdir_config" ] || ! cmp -s "$workdir_config" "$default_config"; then
        echo "Detected suibase.yaml differs from default template, restoring..."
        cp "$default_config" "$workdir_config"
        echo "✓ Restored $workdir suibase.yaml from $default_config"
    else
        echo "✓ $workdir suibase.yaml matches default template"
    fi
}

# Call validation early
validate_workdir_support

# Restore suibase.yaml from default template if corrupted by previous tests
# This is called automatically when __test_common.sh is sourced
restore_suibase_yaml_from_default "$WORKDIR"

# Portable process PID detection (copied from __globals.sh to avoid mutex issues)
get_process_pid() {
  local _PROC="$1"
  local _ARGS="$2"
  local _PID
  # Given a process "string" return the pid as a string.
  # Return NULL if not found.

  # Detect OS for platform-specific ps behavior
  if [[ "$OSTYPE" == "darwin"* ]]; then
    # MacOS 'ps' works differently and does not show the $_ARGS to discern the
    # process, so next best thing is to match $_PROC to end-of-line with "$".
    # shellcheck disable=SC2009
    _PID=$(ps x -o pid,comm | grep "$_PROC$" | grep -v -e grep | { head -n 1; cat >/dev/null 2>&1; } | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | { head -n 1; cat >/dev/null 2>&1; })
  else
    local _TARGET_CMD
    if [ -n "$_ARGS" ]; then
      _TARGET_CMD="$_PROC $_ARGS"
    else
      _TARGET_CMD="$_PROC"
    fi

    # shellcheck disable=SC2009
    _PID=$(ps x -o pid,cmd 2>/dev/null | grep "$_TARGET_CMD" | grep -v grep | { head -n 1; cat >/dev/null 2>&1; } | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | { head -n 1; cat >/dev/null 2>&1; })
  fi

  if [ -n "$_PID" ]; then
    echo "$_PID"
  else
    echo "NULL"
  fi
}

# Test configuration
TEMP_TEST_DIR="/tmp/suibase_walrus_relay_test_$$"

# Simple test failure function
fail() {
    echo "ERROR: $1" >&2
    exit 1
}

# Strip ANSI color codes from input (portable across macOS/Linux)
# Usage: strip_ansi_colors "$input" or echo "$input" | strip_ansi_colors
strip_ansi_colors() {
    if [ $# -eq 0 ]; then
        # Read from stdin - use printf to insert literal ESC character
        sed "s/$(printf '\033')\[[0-9;]*m//g"
    else
        # Process argument
        echo "$1" | sed "s/$(printf '\033')\[[0-9;]*m//g"
    fi
}

# Test setup function - ensures clean environment before starting
setup_clean_environment() {
    # Clean up any stale processes from previous test runs
    cleanup_port_conflicts

    # Clean up stale walrus locks that may have been left by interrupted processes
    local _WALRUS_LOCK="/tmp/.suibase/cli-walrus.lock"
    if [ -d "$_WALRUS_LOCK" ]; then
        if ! lsof "$_WALRUS_LOCK" >/dev/null 2>&1; then
            echo "Removing stale walrus lock directory: $_WALRUS_LOCK"
            rmdir "$_WALRUS_LOCK" 2>/dev/null || true
        fi
    fi

    # Remove any previous test temp directories
    rm -rf /tmp/suibase_walrus_relay_test_* 2>/dev/null || true
}

# Ensure BUILD version of suibase-daemon is used (not precompiled)
ensure_build_daemon() {
    local version_file="$WORKDIRS/common/bin/suibase-daemon-version.yaml"
    local daemon_binary="$WORKDIRS/common/bin/suibase-daemon"

    # Check if both BUILD version file exists AND binary exists
    if [ -f "$version_file" ] && [ -f "$daemon_binary" ] && grep -q 'origin: "built"' "$version_file"; then
        echo "✓ BUILD version of suibase-daemon is already available"
        return 0
    fi

    if [ -f "$version_file" ] && grep -q 'origin: "precompiled"' "$version_file"; then
        # Rebuild daemon for walrus relay features
        "$SUIBASE_DIR/scripts/dev/update-daemon" >/dev/null 2>&1

        # Wait for rebuild to complete with shorter timeout
        local wait_count=0
        while [ $wait_count -lt 15 ]; do
            if [ -f "$version_file" ] && ! grep -q 'origin: "precompiled"' "$version_file"; then
                return 0
            fi
            sleep 1
            wait_count=$((wait_count + 1))
        done
        return 1
    elif [ ! -f "$version_file" ] || [ ! -f "$daemon_binary" ]; then
        # Force rebuild if version file missing OR binary missing
        "$SUIBASE_DIR/scripts/dev/update-daemon"
    fi
}

# Safe daemon start that ensures BUILD version
safe_start_daemon() {
    # Ensure we have BUILD version
    ensure_build_daemon

    # Start the daemon using proper script
    if ! "$SUIBASE_DIR/scripts/dev/is-daemon-running" >/dev/null 2>&1; then
        echo "Starting suibase-daemon..."
        "$SUIBASE_DIR/scripts/dev/start-daemon" >/dev/null 2>&1
    else
        echo "✓ suibase-daemon is already running"
    fi
}

# Test helper functions
setup_test_workdir() {
    local workdir="$1"

    # Use the existing workdir create script if workdir doesn't exist
    if [ ! -d "$WORKDIRS/$workdir" ] || [ ! -f "$WORKDIRS/$workdir/suibase.yaml" ]; then
        echo "Setting up $workdir workdir..."
        "$SUIBASE_DIR/scripts/$workdir" create >/dev/null 2>&1 || true
    fi

    # Initialize the workdir to ensure binaries are downloaded and setup is complete
    echo "Initializing $workdir workdir..."
    "$SUIBASE_DIR/scripts/$workdir" start >/dev/null 2>&1 || true

    # Ensure additional test directories exist
    mkdir -p "$WORKDIRS/$workdir/bin"
    mkdir -p "$WORKDIRS/$workdir/config-default"
}

backup_config_files() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"

    # Ensure TEMP_TEST_DIR is set
    if [ -z "$TEMP_TEST_DIR" ]; then
        TEMP_TEST_DIR="/tmp/suibase_walrus_relay_test_$$"
    fi

    local backup_dir="$TEMP_TEST_DIR/backup_$workdir"

    mkdir -p "$backup_dir"

    # Backup existing config files
    if [ -f "$config_dir/walrus-config.yaml" ]; then
        cp "$config_dir/walrus-config.yaml" "$backup_dir/" || echo "Warning: Failed to backup walrus-config.yaml"
    fi
    if [ -f "$config_dir/relay-config.yaml" ]; then
        cp "$config_dir/relay-config.yaml" "$backup_dir/" || echo "Warning: Failed to backup relay-config.yaml"
    fi
}

restore_config_files() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"

    # Ensure TEMP_TEST_DIR is set
    if [ -z "$TEMP_TEST_DIR" ]; then
        TEMP_TEST_DIR="/tmp/suibase_walrus_relay_test_$$"
    fi

    local backup_dir="$TEMP_TEST_DIR/backup_$workdir"

    # Restore backed up config files (ignore errors to avoid failing cleanup)
    if [ -d "$backup_dir" ]; then
        [ -f "$backup_dir/walrus-config.yaml" ] && cp "$backup_dir/walrus-config.yaml" "$config_dir/" 2>/dev/null || true
        [ -f "$backup_dir/relay-config.yaml" ] && cp "$backup_dir/relay-config.yaml" "$config_dir/" 2>/dev/null || true
    fi
    
    # Clean up temp directory
    rm -rf "$TEMP_TEST_DIR" 2>/dev/null || true
}

# Stop any walrus relay process that might be running
stop_walrus_relay_process() {
    if [ -n "${WALRUS_RELAY_PROCESS_PID:-}" ]; then
        echo "Stopping walrus relay process PID $WALRUS_RELAY_PROCESS_PID"
        kill "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true
        wait "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true
        unset WALRUS_RELAY_PROCESS_PID
    fi
}

cleanup_port_conflicts() {
    local test_port
    test_port=$("$(dirname "${BASH_SOURCE[0]}")/utils/__get-walrus-relay-local-port.sh" "$WORKDIR" 2>&1 || echo "UNKNOWN")
    
    if [ "$test_port" = "UNKNOWN" ]; then
        echo "ERROR in cleanup_port_conflicts: Failed to get walrus relay port for workdir '$WORKDIR'"
        echo "This likely means walrus_relay_local_port is not configured in suibase.yaml"
        echo "Please ensure walrus_relay_local_port is set in $HOME/suibase/workdirs/$WORKDIR/suibase.yaml"
        echo "Example: walrus_relay_local_port: 45802"
        exit 1
    fi
    
    echo "Checking for port conflicts on port $test_port..."

    # Find processes using the port with more robust detection
    local pids
    pids=$(ss -tlnp 2>/dev/null | grep ":$test_port " | grep -o 'pid=[0-9]*' | cut -d= -f2 | sort -u 2>/dev/null || true)

    if [ -n "$pids" ]; then
        echo "Found processes using port $test_port: $pids"
        for pid in $pids; do
            if kill -0 "$pid" 2>/dev/null; then
                # Check if it's actually a walrus-upload-relay process
                if ps -p "$pid" -o cmd= 2>/dev/null | grep -q "walrus-upload-relay"; then
                    echo "Stopping walrus-upload-relay process $pid using port $test_port"
                    kill "$pid" 2>/dev/null || true
                    # Wait for graceful shutdown
                    sleep 5
                    # Force kill if still running
                    if kill -0 "$pid" 2>/dev/null; then
                        echo "Force killing process $pid"
                        kill -9 "$pid" 2>/dev/null || true
                    fi
                fi
            fi
        done

        # Wait for port to be released
        wait_for_port_available "$test_port" 5
    else
        echo "✓ Port $test_port is available"
    fi
}

wait_for_port_available() {
    local port="$1"
    local timeout="${2:-10}"
    local end=$((SECONDS + timeout))

    while [ $SECONDS -lt $end ]; do
        if ! ss -tln 2>/dev/null | grep -q ":$port "; then
            echo "✓ Port $port is now available"
            return 0
        fi
        sleep 0.5
    done

    echo "⚠ Port $port still in use after ${timeout}s timeout"
    return 1
}

wait_for_process_ready() {
    local port="$1"
    local endpoint="${2:-/v1/tip-config}"
    local timeout="${3:-30}"
    local end=$((SECONDS + timeout))

    echo "Waiting for process to be ready on port $port (endpoint: $endpoint, timeout: ${timeout}s)..."

    while [ $SECONDS -lt $end ]; do
        # First check if port is listening
        if ss -tln 2>/dev/null | grep -q ":$port "; then
            # Then check if endpoint responds
            if curl -s -m 2 "http://localhost:$port$endpoint" >/dev/null 2>&1; then
                echo "✓ Process ready and responding on port $port"
                return 0
            fi
        fi
        sleep 0.5
    done

    echo "✗ Process not ready after ${timeout}s timeout"
    return 1
}

# Override cleanup function
cleanup() {
    # Call original cleanup if it exists
    type original_cleanup >/dev/null 2>&1 && original_cleanup
}

# Test assertion helpers
assert_binary_exists() {
    local workdir="$1"
    local binary_path="$WORKDIRS/$workdir/bin/walrus-upload-relay"

    if [ ! -f "$binary_path" ]; then
        fail "walrus-upload-relay binary not found at $binary_path"
    fi

    if [ ! -x "$binary_path" ]; then
        fail "walrus-upload-relay binary not executable at $binary_path"
    fi

    echo "✓ walrus-upload-relay binary exists and is executable at $binary_path"
}

assert_process_running() {
    local pid="$1"
    local process_name="$2"

    if [ -z "$pid" ]; then
        fail "$process_name PID is empty"
    fi

    if ! kill -0 "$pid" 2>/dev/null; then
        fail "$process_name process (PID $pid) is not running"
    fi

    echo "✓ $process_name process is running with PID $pid"
}

assert_config_file_exists() {
    local workdir="$1"
    local config_file="$2"
    local config_path="$WORKDIRS/$workdir/config-default/$config_file"

    if [ ! -f "$config_path" ]; then
        fail "$config_file not found at $config_path"
    fi

    echo "✓ $config_file exists at $config_path"
}

# Portable walrus relay process checking
check_walrus_process_stopped() {
    local workdir="$1"
    local workdir_prefix
    case "$workdir" in
        "testnet") workdir_prefix="t" ;;
        "mainnet") workdir_prefix="m" ;;
        *) workdir_prefix="${workdir:0:1}" ;;
    esac

    local workdir_binary="$WORKDIRS/$workdir/bin/${workdir_prefix}walrus-upload-relay"
    local pid
    pid=$(get_process_pid "$workdir_binary")

    if [ "$pid" = "NULL" ]; then
        echo "✓ ${workdir_prefix}walrus-upload-relay process is stopped"
        return 0
    else
        echo "✗ ${workdir_prefix}walrus-upload-relay process still running (PID: $pid)"
        return 1
    fi
}

check_walrus_process_running() {
    local workdir="$1"
    local workdir_prefix
    case "$workdir" in
        "testnet") workdir_prefix="t" ;;
        "mainnet") workdir_prefix="m" ;;
        *) workdir_prefix="${workdir:0:1}" ;;
    esac

    local workdir_binary="$WORKDIRS/$workdir/bin/${workdir_prefix}walrus-upload-relay"
    local pid
    pid=$(get_process_pid "$workdir_binary")

    if [ "$pid" != "NULL" ]; then
        echo "✓ ${workdir_prefix}walrus-upload-relay process is running (PID: $pid)"
        return 0
    else
        echo "✗ ${workdir_prefix}walrus-upload-relay process is not running"
        return 1
    fi
}

# Generic service status waiting utility
wait_for_service_status() {
    local command="$1"           # Full command to execute (e.g., "testnet wal-relay status")
    local expected_status="$2"   # Expected status or pipe-separated list (e.g., "OK|INITIALIZING")
    local status_label="$3"      # Label to look for in output (e.g., "Walrus Relay")
    local timeout_seconds="${4:-15}" # Optional timeout, default 15 seconds
    local verbose="${5:-false}"  # Optional verbose mode

    local start_time=$SECONDS
    local end_time=$((start_time + timeout_seconds))
    local attempt=0

    if [ "$verbose" = "true" ]; then
        echo "Waiting for $status_label status to be [$expected_status] (timeout: ${timeout_seconds}s)"
    fi

    while [ $SECONDS -lt $end_time ]; do
        attempt=$((attempt + 1))

        # Execute the command and capture output
        local output
        output=$($command 2>&1) || true

        # Extract the line for the status label and check if any expected status is contained
        # Strip ANSI color codes before parsing
        local status_line
        status_line=$(strip_ansi_colors "$output" | grep "^$status_label" | head -n1)

        if [ -n "$status_line" ]; then
            # Get everything after the first colon
            local status_part
            status_part=$(echo "$status_line" | sed 's/^[^:]*: *//')

            # Check if any of the expected statuses is contained in the status part
            local found_match=false
            IFS='|' read -ra STATUS_LIST <<< "$expected_status"
            for status in "${STATUS_LIST[@]}"; do
                if echo "$status_part" | grep -q "$status"; then
                    found_match=true
                    break
                fi
            done

            if [ "$found_match" = true ]; then
                if [ "$verbose" = "true" ]; then
                    echo "✓ $status_label status line contains expected status after ${attempt} attempts ($(($SECONDS - start_time))s)"
                fi
                return 0
            fi

            if [ "$verbose" = "true" ]; then
                echo "  Attempt $attempt: $status_label status part is [$status_part], waiting..."
            fi
        else
            if [ "$verbose" = "true" ]; then
                echo "  Attempt $attempt: Could not parse $status_label status from output"
                echo "  Command output: $output"
            fi
        fi

        sleep 1
    done

    # Timeout reached
    local final_output
    final_output=$($command 2>&1) || true
    local final_status_line
    final_status_line=$(strip_ansi_colors "$final_output" | grep "^$status_label" | head -n1)
    local final_status_part
    if [ -n "$final_status_line" ]; then
        final_status_part=$(echo "$final_status_line" | sed 's/^[^:]*: *//')
    else
        final_status_part="UNKNOWN"
    fi

    echo "✗ Timeout: $status_label status did not reach [$expected_status] within ${timeout_seconds}s"
    echo "  Final status part: [${final_status_part}] after $attempt attempts"
    echo "  Final command output:"
    echo "$final_output" | sed 's/^/    /'

    return 1
}

# Walrus relay specific wrapper function
wait_for_walrus_relay_status() {
    local workdir="$1"           # testnet, mainnet, etc.
    local expected_status="$2"   # Expected status (e.g., "OK", "INITIALIZING", "OK|INITIALIZING")
    local timeout_seconds="${3:-15}" # Optional timeout, default 15 seconds
    local verbose="${4:-false}"  # Optional verbose mode

    local command="$workdir wal-relay status"
    wait_for_service_status "$command" "$expected_status" "Walrus Relay" "$timeout_seconds" "$verbose"
}

# Wait for daemon to be running and ready
wait_for_daemon_running() {
    local timeout_seconds="${1:-15}" # Optional timeout, default 15 seconds
    local verbose="${2:-false}"      # Optional verbose mode

    local start_time=$SECONDS
    local end_time=$((start_time + timeout_seconds))
    local attempt=0

    if [ "$verbose" = "true" ]; then
        echo "Waiting for suibase-daemon to be running (timeout: ${timeout_seconds}s)"
    fi

    while [ $SECONDS -lt $end_time ]; do
        attempt=$((attempt + 1))

        if "$SUIBASE_DIR/scripts/dev/is-daemon-running" >/dev/null 2>&1; then
            if [ "$verbose" = "true" ]; then
                echo "✓ suibase-daemon is running after ${attempt} attempts ($(($SECONDS - start_time))s)"
            fi
            return 0
        fi

        if [ "$verbose" = "true" ]; then
            echo "  Attempt $attempt: daemon not running yet, waiting..."
        fi

        sleep 1
    done

    echo "✗ Timeout: suibase-daemon not running within ${timeout_seconds}s after $attempt attempts"
    return 1
}

# Wait for daemon to be stopped
wait_for_daemon_stopped() {
    local timeout_seconds=100 # Fixed timeout of 100 seconds to account for 90s daemon shutdown timeout
    local verbose="${1:-false}" # Optional verbose mode

    local start_time=$SECONDS
    local end_time=$((start_time + timeout_seconds))
    local attempt=0

    if [ "$verbose" = "true" ]; then
        echo "Waiting for suibase-daemon to stop (timeout: ${timeout_seconds}s)"
    fi

    while [ $SECONDS -lt $end_time ]; do
        attempt=$((attempt + 1))

        if ! "$SUIBASE_DIR/scripts/dev/is-daemon-running" >/dev/null 2>&1; then
            if [ "$verbose" = "true" ]; then
                echo "✓ suibase-daemon stopped after ${attempt} attempts ($(($SECONDS - start_time))s)"
            fi
            return 0
        fi

        if [ "$verbose" = "true" ]; then
            echo "  Attempt $attempt: daemon still running, waiting..."
        fi

        sleep 1
    done

    echo "✗ Timeout: suibase-daemon still running after ${timeout_seconds}s and $attempt attempts"
    return 1
}

# Wait for walrus process to be stopped
wait_for_process_stopped() {
    local workdir="$1"               # testnet, mainnet, etc.
    local timeout_seconds="${2:-10}" # Optional timeout, default 10 seconds
    local verbose="${3:-false}"      # Optional verbose mode

    local start_time=$SECONDS
    local end_time=$((start_time + timeout_seconds))
    local attempt=0

    if [ "$verbose" = "true" ]; then
        echo "Waiting for ${workdir} walrus process to stop (timeout: ${timeout_seconds}s)"
    fi

    while [ $SECONDS -lt $end_time ]; do
        attempt=$((attempt + 1))

        if check_walrus_process_stopped "$workdir" >/dev/null 2>&1; then
            if [ "$verbose" = "true" ]; then
                echo "✓ ${workdir} walrus process stopped after ${attempt} attempts ($(($SECONDS - start_time))s)"
            fi
            return 0
        fi

        if [ "$verbose" = "true" ]; then
            echo "  Attempt $attempt: walrus process still running, waiting..."
        fi

        sleep 1
    done

    echo "✗ Timeout: ${workdir} walrus process still running after ${timeout_seconds}s and $attempt attempts"
    return 1
}

export -f get_process_pid check_walrus_process_stopped check_walrus_process_running
export -f setup_clean_environment ensure_build_daemon safe_start_daemon setup_test_workdir backup_config_files restore_config_files stop_walrus_relay_process
export -f cleanup_port_conflicts wait_for_port_available wait_for_process_ready
export -f assert_binary_exists assert_process_running assert_config_file_exists
export -f wait_for_service_status wait_for_walrus_relay_status wait_for_daemon_running wait_for_daemon_stopped wait_for_process_stopped
export -f restore_suibase_yaml_from_default
export -f fail strip_ansi_colors