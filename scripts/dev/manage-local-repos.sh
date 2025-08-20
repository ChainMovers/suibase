#!/bin/bash

echo "🛠️  Starting manage-local-repos.sh..."

# manage-local-repos.sh - Unified local repository management for Claude Code efficiency
# Manages Sui, sui-rust-sdk, and Walrus repositories for faster searches, reduced tokens, local-first

set -euo pipefail

# Repository configurations - simplified hardcoded approach
declare -A REPO_URLS=(
    ["sui"]="https://github.com/MystenLabs/sui.git"
    ["sui-rust-sdk"]="https://github.com/MystenLabs/sui-rust-sdk.git"
    ["walrus"]="https://github.com/MystenLabs/walrus.git"
)

declare -A REPO_DIRS=(
    ["sui"]="sui-reference-main"
    ["sui-rust-sdk"]="sui-rust-sdk-main"
    ["walrus"]="walrus-reference-main"
)

declare -A REPO_ENV_VARS=(
    ["sui"]="SUI_REFERENCE_PATH"
    ["sui-rust-sdk"]="SUI_RUST_SDK_PATH"
    ["walrus"]="WALRUS_REFERENCE_PATH"
)

declare -A REPO_SPARSE_PATHS=(
    ["sui"]="crates/,external-crates/,consensus/,sui-execution/,sdk/,bridge/,docs/,*.md,*.toml,*.lock"
    ["sui-rust-sdk"]="crates/,*.md,*.toml,*.lock,*.proto"
    ["walrus"]="crates/walrus-upload-relay/,crates/walrus-core/,crates/walrus-sui/,crates/walrus-storage-node-client/,contracts/,*.md,*.toml,*.lock"
)

readonly CLONE_DEPTH=100
readonly MIN_UPDATE_HOURS=6

# Global variables for current repository context
CURRENT_REPO_URL=""
CURRENT_REPO_DIR=""
CURRENT_REPO_NAME=""
declare -a CURRENT_SPARSE_PATHS=()

# Parse repository configuration
parse_repo_config() {
    local repo_name="$1"

    # Get configuration from arrays
    local repo_url="${REPO_URLS[$repo_name]}"
    local repo_dir="${REPO_DIRS[$repo_name]}"
    local env_var="${REPO_ENV_VARS[$repo_name]}"
    local sparse_paths="${REPO_SPARSE_PATHS[$repo_name]}"

    # Use environment variable or default path
    local default_dir="$HOME/repos/$repo_dir"
    local resolved_dir

    # Check if environment variable exists and is not empty
    if [[ -n "$env_var" ]] && [[ -v "$env_var" ]] && [[ -n "${!env_var}" ]]; then
        resolved_dir="${!env_var}"
        # Expand tilde if present
        resolved_dir="${resolved_dir/#~/$HOME}"
    else
        resolved_dir="$default_dir"
    fi

    # Convert sparse paths to array
    local -a sparse_array
    IFS=',' read -ra sparse_array <<< "$sparse_paths"

    # Set global variables
    CURRENT_REPO_URL="$repo_url"
    CURRENT_REPO_DIR="$resolved_dir"
    CURRENT_REPO_NAME="$repo_name"
    CURRENT_SPARSE_PATHS=("${sparse_array[@]}")
}

# Logging with minimal overhead
log_info() { echo "[INFO] $*"; }
log_warn() { echo "[WARN] $*" >&2; }
log_error() { echo "[ERROR] $*" >&2; }

# Performance: Quick existence check
repo_exists() {
    [[ -d "$CURRENT_REPO_DIR/.git" ]]
}

# Performance: Fast status check without network
repo_needs_update() {
    if ! repo_exists; then
        return 0  # Needs init
    fi

    # Check if last update was recent enough
    local last_fetch_file="$CURRENT_REPO_DIR/.git/FETCH_HEAD"
    if [[ ! -f "$last_fetch_file" ]]; then
        return 0  # Never fetched
    fi

    local last_fetch_time
    last_fetch_time=$(stat -c "%Y" "$last_fetch_file" 2>/dev/null || echo 0)
    local current_time
    current_time=$(date +%s)
    local hours_since_update
    hours_since_update=$(( (current_time - last_fetch_time) / 3600 ))

    [[ $hours_since_update -ge $MIN_UPDATE_HOURS ]]
}

# Performance: Validate without expensive operations
repo_is_healthy() {
    if ! repo_exists; then
        return 1
    fi

    # Quick integrity checks
    git -C "$CURRENT_REPO_DIR" rev-parse HEAD >/dev/null 2>&1 && \
    git -C "$CURRENT_REPO_DIR" status --porcelain >/dev/null 2>&1
}

# Efficiency: Sparse checkout for minimal disk usage
setup_sparse_checkout() {
    local repo_dir="$1"

    git -C "$repo_dir" config core.sparseCheckout true

    # Write sparse-checkout file
    {
        for path in "${CURRENT_SPARSE_PATHS[@]}"; do
            echo "$path"
        done
    } > "$repo_dir/.git/info/sparse-checkout"

    git -C "$repo_dir" read-tree -m -u HEAD
}

# Performance: Optimized clone for Claude Code usage
init_repository() {
    # Create parent directory if needed
    mkdir -p "$(dirname "$CURRENT_REPO_DIR")"

    # Try main branch first, fall back to master
    if ! git clone \
        --depth "$CLONE_DEPTH" \
        --single-branch \
        --branch main \
        --filter=blob:none \
        "$CURRENT_REPO_URL" \
        "$CURRENT_REPO_DIR" 2>/dev/null; then

        # Try master branch instead
        git clone \
            --depth "$CLONE_DEPTH" \
            --single-branch \
            --branch master \
            --filter=blob:none \
            "$CURRENT_REPO_URL" \
            "$CURRENT_REPO_DIR"
    fi

    # Efficiency: Setup sparse checkout immediately
    setup_sparse_checkout "$CURRENT_REPO_DIR"

    # Performance: Configure for faster operations
    git -C "$CURRENT_REPO_DIR" config gc.auto 0
    git -C "$CURRENT_REPO_DIR" config fetch.prune true
    git -C "$CURRENT_REPO_DIR" config remote.origin.prune true

    # Build search index for faster Claude queries
    build_search_index >/dev/null 2>&1
    echo " ✅"
}

# Performance: Incremental updates only
update_repository() {
    if ! repo_exists; then
        init_repository
        return
    fi

    # Updating repository

    # Determine the default branch (main or master)
    local default_branch
    if git -C "$CURRENT_REPO_DIR" rev-parse --verify origin/main >/dev/null 2>&1; then
        default_branch="main"
    else
        default_branch="master"
    fi

    # Performance: Fetch only what's needed
    git -C "$CURRENT_REPO_DIR" fetch --depth "$CLONE_DEPTH" origin "$default_branch"

    # Performance: Fast-forward merge only
    local current_head
    current_head=$(git -C "$CURRENT_REPO_DIR" rev-parse HEAD)

    git -C "$CURRENT_REPO_DIR" reset --hard "origin/$default_branch"

    local new_head
    new_head=$(git -C "$CURRENT_REPO_DIR" rev-parse HEAD)

    if [[ "$current_head" != "$new_head" ]]; then
        # Rebuild index only if content changed
        build_search_index >/dev/null 2>&1
    fi
    echo " ✅"
}

# Performance: Pre-build search indices for Claude Code
build_search_index() {
    if ! command -v rg >/dev/null 2>&1; then
        log_warn "ripgrep not found - search performance may be reduced"
        return
    fi

    # Building search index for faster Claude Code queries

    # Performance: Pre-cache common search patterns
    local index_cache="$CURRENT_REPO_DIR/.claude_search_cache"
    mkdir -p "$index_cache"

    # Cache important file lists for instant access
    rg --files "$CURRENT_REPO_DIR" --type rust > "$index_cache/rust_files.txt" 2>/dev/null || true
    rg --files "$CURRENT_REPO_DIR" --type toml > "$index_cache/toml_files.txt" 2>/dev/null || true
    rg --files "$CURRENT_REPO_DIR" --glob "*.md" > "$index_cache/doc_files.txt" 2>/dev/null || true
    rg --files "$CURRENT_REPO_DIR" --glob "*.proto" > "$index_cache/proto_files.txt" 2>/dev/null || true

    # Cache common patterns for development
    rg "pub struct" "$CURRENT_REPO_DIR" --type rust --line-number > "$index_cache/structs.txt" 2>/dev/null || true
    rg "pub enum" "$CURRENT_REPO_DIR" --type rust --line-number > "$index_cache/enums.txt" 2>/dev/null || true
    rg "pub fn" "$CURRENT_REPO_DIR" --type rust --line-number > "$index_cache/functions.txt" 2>/dev/null || true
    rg "async fn" "$CURRENT_REPO_DIR" --type rust --line-number > "$index_cache/async_functions.txt" 2>/dev/null || true
    rg "service.*{|rpc.*(" "$CURRENT_REPO_DIR" --glob "*.proto" --line-number > "$index_cache/grpc_services.txt" 2>/dev/null || true

    # Search index built
}

# Efficiency: Comprehensive validation without expensive operations
validate_repository() {
    local exit_code=0

    log_info "Validating $CURRENT_REPO_NAME repository..."

    # Check existence
    if ! repo_exists; then
        log_error "Repository does not exist at: $CURRENT_REPO_DIR"
        log_info "Run: $0 $CURRENT_REPO_NAME init"
        return 1
    fi

    # Check health
    if ! repo_is_healthy; then
        log_error "Repository appears corrupted"
        ((exit_code++))
    fi

    # Check remote
    local remote_url
    remote_url=$(git -C "$CURRENT_REPO_DIR" remote get-url origin 2>/dev/null || echo "")
    if [[ "$remote_url" != "$CURRENT_REPO_URL" ]]; then
        log_error "Remote URL mismatch: expected $CURRENT_REPO_URL, got $remote_url"
        ((exit_code++))
    fi

    # Check sparse checkout
    if ! git -C "$CURRENT_REPO_DIR" config core.sparseCheckout >/dev/null 2>&1; then
        log_warn "Sparse checkout not configured - repository uses more disk space"
    fi

    # Check search index
    local index_cache="$CURRENT_REPO_DIR/.claude_search_cache"
    if [[ ! -d "$index_cache" ]] || [[ ! -f "$index_cache/rust_files.txt" ]]; then
        log_warn "Search index missing - Claude Code queries may be slower"
        log_info "Run: $0 $CURRENT_REPO_NAME update (to rebuild index)"
    fi

    # Performance stats
    local repo_size
    repo_size=$(du -sh "$CURRENT_REPO_DIR" 2>/dev/null | cut -f1 || echo "unknown")
    local file_count
    file_count=$(find "$CURRENT_REPO_DIR" -type f | wc -l | tr -d ' ')
    local last_update
    last_update=$(git -C "$CURRENT_REPO_DIR" log -1 --format="%cr" 2>/dev/null || echo "unknown")

    log_info "✓ Repository size: $repo_size"
    log_info "✓ File count: $file_count"
    log_info "✓ Last commit: $last_update"

    if [[ $exit_code -eq 0 ]]; then
        log_info "✓ Repository validation passed"
    else
        log_error "Repository validation failed with $exit_code issues"
    fi

    return $exit_code
}

# Efficiency: Quick status for pre-session hooks
status_check() {
    if ! repo_exists; then
        echo "❌ $CURRENT_REPO_NAME not initialized"
        echo "   Run: $0 $CURRENT_REPO_NAME init"
        return 1
    fi

    if ! repo_is_healthy; then
        echo "⚠️  $CURRENT_REPO_NAME corrupted"
        echo "   Run: $0 $CURRENT_REPO_NAME init"
        return 1
    fi

    if repo_needs_update; then
        echo "🔄 $CURRENT_REPO_NAME needs update"
        echo "   Run: $0 $CURRENT_REPO_NAME update"
        return 2
    fi

    echo "✅ $CURRENT_REPO_NAME ready"
    return 0
}

# Efficiency: Clean up to free space
clean_repository() {
    if ! repo_exists; then
        log_warn "Repository does not exist - nothing to clean"
        return 0
    fi

    log_info "Cleaning $CURRENT_REPO_NAME repository to optimize performance..."

    # Clean Git data
    git -C "$CURRENT_REPO_DIR" gc --aggressive --prune=now
    git -C "$CURRENT_REPO_DIR" remote prune origin

    # Clean search cache
    rm -rf "$CURRENT_REPO_DIR/.claude_search_cache"

    # Rebuild optimized index
    build_search_index

    log_info "✓ Repository cleaned and optimized"
}

# Auto-mode: Detect and perform needed action, then show status
auto_action() {
    if ! repo_exists; then
        init_repository
    elif ! repo_is_healthy; then
        log_warn "Repository corrupted - reinitializing"
        rm -rf "$CURRENT_REPO_DIR"
        init_repository
    elif repo_needs_update; then
        update_repository
    else
        echo " ready"
    fi
}

# Multi-repository operations
all_repos_action() {
    local action="$1"
    local overall_status=0

    for repo_name in "${!REPO_URLS[@]}"; do
        parse_repo_config "$repo_name"

        case "$action" in
            validate)
                if ! validate_repository; then
                    overall_status=1
                fi
                ;;
            clean)
                clean_repository
                ;;
            *)
                # Default to auto action for all repos
                echo -n "✅ $repo_name"
                if ! auto_action; then
                    overall_status=1
                fi
                ;;
        esac
    done

    return $overall_status
}

# Usage information optimized for developers
usage() {
    cat << EOF
Usage: $0 [command]

DESCRIPTION:
  Manages local Sui and Walrus repositories for Claude Code optimization.
  By default, ensures all repositories are ready (init/update as needed) and shows status.

REPOSITORIES MANAGED:
  - sui: Sui main repository (gRPC API, core types)
  - sui-rust-sdk: Sui Rust SDK (proto files, client types)
  - walrus: Walrus decentralized storage (upload relay, core types)

COMMANDS:
  (default)             Ensure all repositories are ready, then show status
  validate              Comprehensive health check and diagnostics for all repos
  clean                 Clean and optimize all repositories

ENVIRONMENT VARIABLES:
  SUI_REFERENCE_PATH    Override sui repository path (default: ~/repos/sui-reference-main)
  SUI_RUST_SDK_PATH     Override sui-rust-sdk path (default: ~/repos/sui-rust-sdk-main)
  WALRUS_REFERENCE_PATH Override walrus repository path (default: ~/repos/walrus-reference-main)

OPTIMIZATION:
  - Shallow clone with depth=$CLONE_DEPTH for faster operations
  - Sparse checkout for minimal disk usage
  - Pre-built search indices for faster Claude Code queries
  - Incremental updates only when needed
  - Always ensures repositories are ready and shows final status

EXAMPLES:
  $0                             # Ensure all repositories are ready
  $0 validate                    # Comprehensive health check for all repos
  $0 >/dev/null && echo "All ready" || echo "Needs attention"
EOF
}

# Main execution with error handling
main() {
    local command="${1:-auto}"

    # Handle help requests
    if [[ "$command" == "help" || "$command" == "--help" || "$command" == "-h" ]]; then
        usage
        exit 0
    fi

    # Execute command on all repositories
    case "$command" in
        validate|check)
            all_repos_action "validate"
            ;;
        clean|optimize)
            all_repos_action "clean"
            ;;
        auto|*)
            # Default to auto action for all repos
            all_repos_action "auto"
            ;;
    esac
    echo
}

# Performance: Exit fast on common cases
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi