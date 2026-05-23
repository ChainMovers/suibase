# shellcheck shell=bash

# Intended to be sourced only in __workdir-exec.sh

# Code that does publish modules to a Sui network

publish_all() {

  local _PASSTHRU_OPTIONS="${*}"

  if [ -z "$MOVE_TOML_PACKAGE_NAME" ]; then
    echo "suibase: Package name could not be found"
    exit 1
  fi

  # Add default --gas-budget if not specified.
  # shellcheck disable=SC2086
  if ! has_param "" "--gas-budget" $_PASSTHRU_OPTIONS; then
    _PASSTHRU_OPTIONS="$_PASSTHRU_OPTIONS --gas-budget 500000000"
  fi

  # Add --json, but only if not already specified by the caller.
  # shellcheck disable=SC2086
  if ! has_param "" "--json" $_PASSTHRU_OPTIONS; then
    _PASSTHRU_OPTIONS="$_PASSTHRU_OPTIONS --json"
  fi

  # Add --with-unpublished-dependencies if not already specified and
  # local unpublished dependencies are found in the Move.toml
  if ! has_param "" "--with-unpublished-dependencies"; then
    if has_unpublished_dependencies "$MOVE_TOML_DIR"; then
      _PASSTHRU_OPTIONS="$_PASSTHRU_OPTIONS --with-unpublished-dependencies"
    fi
  fi

  # Do a pre publication handshake with the suibase-daemon.
  # On success, will get the global PACKAGE_UUID variable set.
  # On failure, the script will exit_error.
  do_suibase_daemon_pre_publish "$MOVE_TOML_DIR" "$MOVE_TOML_PACKAGE_NAME"

  echo "Package name=[$MOVE_TOML_PACKAGE_NAME]"

  local _SUB_INSTALL_DIR="$MOVE_TOML_PACKAGE_NAME/$PACKAGE_UUID/$PACKAGE_TIMESTAMP"
  echo "Script outputs in ~/suibase/workdirs/$WORKDIR_NAME/published-data/$_SUB_INSTALL_DIR"

  INSTALL_DIR="$PUBLISHED_DATA_DIR/$_SUB_INSTALL_DIR"

  mkdir -p "$INSTALL_DIR"

  publish_clear_output "$INSTALL_DIR"

  sync_client_yaml

  # Auto-sync Move.toml [environments] chain_ids against the workdir's
  # live chain identifier. `localnet regen` produces a fresh genesis
  # (new chain_id) each time, so any hardcoded value in committed
  # Move.toml files becomes stale and sui-package-alt rejects the
  # publish. This loop reads the current chain_id once and rewrites
  # only the entries that drifted; it does NOT touch the file when
  # values already match (no spurious mtime churn, no spurious diffs
  # in the user's working tree).
  #
  # Scope: localnet ONLY. testnet/mainnet/devnet have stable chain_ids
  # and any mismatch there is a real configuration error the user
  # should resolve consciously — silently rewriting committed Move.toml
  # under those workdirs (when the proxy happens to be mis-routed, or
  # a load-balancer answered with a different chain) is a surprise.
  # The walk includes the package being published plus all transitive
  # `local = "..."` deps in [dependencies] AND [dev-dependencies].
  if should_auto_sync_chainid "$WORKDIR_NAME"; then
    local _LIVE_CHAIN_ID
    _LIVE_CHAIN_ID=$(get_current_chain_id)
    if [ -n "$_LIVE_CHAIN_ID" ]; then
      sync_movetoml_workdir_chainids "$MOVE_TOML_DIR/Move.toml" "$WORKDIR_NAME" "$_LIVE_CHAIN_ID"
      sync_local_deps_chainids "$MOVE_TOML_DIR/Move.toml" "$WORKDIR_NAME" "$_LIVE_CHAIN_ID"
    elif grep -q "^\[environments\]" "$MOVE_TOML_DIR/Move.toml" 2>/dev/null; then
      # Move.toml has the new-style [environments] table but we
      # couldn't determine the live chain_id (proxy down, disabled,
      # or returned garbage). Warn the user so an upcoming
      # chain_id-mismatch error has context — sui-package-alt's
      # error message points at the file, not the resolver.
      echo "Warning: could not auto-sync Move.toml chain_id (proxy unreachable or returned an invalid response)." >&2
      echo "         If 'sui client publish' fails with a chain_id mismatch, run 'localnet status' to verify the proxy is up." >&2
    fi
  fi

  # Build the Move package for publication.
  echo "Will publish using sui client for $WORKDIR_NAME. Command line is:"

  local _CMD="$SUI_EXEC client publish --install-dir \"$INSTALL_DIR\" \"$MOVE_TOML_DIR\" $_PASSTHRU_OPTIONS 2>&1 1>$INSTALL_DIR/publish-output.json"

  local _CMD_TO_DISPLAY=$_CMD

  # For display purpose, replace $SUI_EXEC with user-friendly $SUI_SCRIPT (e.g. "lsui").
  # TODO Code this without using external command.
  _CMD_TO_DISPLAY=$(echo "$_CMD_TO_DISPLAY" | sed "s|$SUI_EXEC|$SUI_SCRIPT|g")

  echo "$_CMD_TO_DISPLAY"
  # Execute $CMD
  echo "=================== Sui client output ===================="
  eval "$_CMD"
  #  TODO Investigate problem with exit status here...

  # Create the created_objects.json file.
  update_SUI_PUBLISH_TXDIGEST "$INSTALL_DIR"
  if [ -n "$SUI_PUBLISH_TXDIGEST" ]; then
    process_object_changes "$INSTALL_DIR"
  fi

  # Load back the package-id.json from the file for validation
  local _ID_PACKAGE
  if [ -f "$INSTALL_DIR/package-id.json" ]; then
    _ID_PACKAGE=$(sed 's/\[//g; s/\]//g; s/"//g;' "$INSTALL_DIR/package-id.json")
  fi

  if [ -z "$_ID_PACKAGE" ]; then
    cat "$INSTALL_DIR/publish-output.json"
  fi

  if [ -z "$_ID_PACKAGE" ]; then
    echo "======================= Summary =========================="
    setup_error "Publication failed."
  fi

  # Test the publication by retreiving object information from the network
  # using that parsed package id.
  echo "================ Verification on Network ================="

  # Retry for up to 30 seconds to allow for the propagation time of information to the RPC nodes.
  # Check no more than once per second.
  local _RETRY_COUNT=0
  local _RETRY_MAX=30
  local _RETRY_DELAY=1
  local _VERIFIED=false

  if [ "$WORKDIR_NAME" != "localnet" ]; then
    sleep $_RETRY_DELAY
  fi

  while [ $_RETRY_COUNT -lt $_RETRY_MAX ]; do
    _RETRY_COUNT=$((_RETRY_COUNT + 1))
    local _ID_PACKAGE_INFO
    _ID_PACKAGE_INFO=$($SUI_EXEC client object "$_ID_PACKAGE" | grep -i "package")
    if [ -n "$_ID_PACKAGE_INFO" ]; then
      _VERIFIED=true
      break
    else
      echo "suibase: Verification attempt $_RETRY_COUNT of $_RETRY_MAX"
      sleep $_RETRY_DELAY
    fi
  done

  if [ "$_VERIFIED" = false ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "Could not confirm package is on the network for packageId=$_ID_PACKAGE"
  else
    echo "suibase: Verification completed. The package is on the network."
  fi

  # Update the 'latest' symlink.
  update_latest_symlinks

  # _ID_PACKAGE_NO_OX
  local _ID_PACKAGE_FOR_LINK
  _ID_PACKAGE_FOR_LINK=$(echo "$_ID_PACKAGE" | sed 's/0x//g')
  local _WORKDIR_NAME_FOR_LINK="$WORKDIR_NAME"
  if [ "$WORKDIR_NAME" = "localnet" ]; then
    _WORKDIR_NAME_FOR_LINK="local"
  fi

  echo "======================= Summary =========================="
  echo "Publication Successful"
  echo "Package ID=[$_ID_PACKAGE]"
  echo "Package ID also in [~/suibase/workdirs/$WORKDIR_NAME/published-data/$MOVE_TOML_PACKAGE_NAME/most-recent/package-id.json]"
  echo "Created objects in [~/suibase/workdirs/$WORKDIR_NAME/published-data/$MOVE_TOML_PACKAGE_NAME/most-recent/created-objects.json]"
  echo "Complete output in [~/suibase/workdirs/$WORKDIR_NAME/published-data/$_SUB_INSTALL_DIR/publish-output.json]"

  if [ "${CFG_sui_explorer_enabled:?}" = "true" ] && [ "${CFG_sui_explorer_host_ip:?}" != "~" ]; then
    echo "==================== Explorer Links ======================"
    # Build the URL using the yaml config. Example of config:
    #
    #   sui_explorer_enabled: true
    #   sui_explorer_scheme: "http://"
    #   sui_explorer_host_ip: "localhost"
    #   sui_explorer_port_number: 44380
    #   sui_explorer_object_path: "/object/{ID}"
    #   sui_explorer_txn_path: "/txblock/{ID}"
    #
    local _URL_BASE
    if [ -n "${CFG_sui_explorer_scheme}" ] && [ "${CFG_sui_explorer_scheme}" != "~" ]; then
      _URL_BASE="${CFG_sui_explorer_scheme}${CFG_sui_explorer_host_ip:?}"
    else
      _URL_BASE="http://${CFG_sui_explorer_host_ip:?}"
    fi

    if [ -n "${CFG_sui_explorer_port_number}" ] && [ "${CFG_sui_explorer_port_number}" != "~" ]; then
      _URL_BASE="${_URL_BASE}:${CFG_sui_explorer_port_number:?}"
    fi

    if [ -n "${CFG_sui_explorer_package_path:?}" ] && [ -n "$_ID_PACKAGE_FOR_LINK" ]; then
      local _URL_PATH="${CFG_sui_explorer_package_path//\{ID\}/$_ID_PACKAGE_FOR_LINK}"
      echo "Package [${_URL_BASE}${_URL_PATH}]"
    fi

    if [ -n "${CFG_sui_explorer_txn_path}" ] && [ -n "$SUI_PUBLISH_TXDIGEST" ]; then
      local _URL_PATH="${CFG_sui_explorer_txn_path//\{ID\}/$SUI_PUBLISH_TXDIGEST}"
      echo "TxBlock [${_URL_BASE}${_URL_PATH}]"
    fi
  fi

  # Push new information to suibase-daemon.
  do_suibase_daemon_post_publish "$MOVE_TOML_DIR" "$MOVE_TOML_PACKAGE_NAME" "$PACKAGE_UUID" "$PACKAGE_TIMESTAMP" "$_ID_PACKAGE"
}
export -f publish_all

export SUI_PUBLISH_TXDIGEST=""
update_SUI_PUBLISH_TXDIGEST() {
  local _INSTALL_DIR="$1"
  unset SUI_PUBLISH_TXDIGEST
  local _block_level=0
  SUI_PUBLISH_TXDIGEST=$(
    cat "$_INSTALL_DIR/publish-output.json" |
      while read -r line || [ -n "$line" ]; do
        # Increment _block_level when '{' is found anywhere in the line.
        if [[ $line == *"{"* ]]; then
          _block_level=$((_block_level + 1))
        fi
        # Decrement _block_level when '}' is found anywhere in the line.
        if [[ $line == *"}"* ]]; then
          _block_level=$((_block_level - 1))
        fi
        if [ $_block_level -eq 1 ]; then
          if [[ $line == *"\"digest\":"* ]]; then
            local _RESULT
            _RESULT=$(echo "$line" | awk -F'"' '{print $4}')
            echo "$_RESULT"
            break
          fi
        fi
      done
  )
}
export -f update_SUI_PUBLISH_TXDIGEST

process_object_changes() {
  local _INSTALL_DIR="$1"

  local _first_object_created=true
  local _block_level=0

  # Iterate every element, which have its fields delimitated by { and }.
  # The fields to be check are when _block_level=1
  local _TYPE=""
  local _PACKAGE_ID=""
  local _OBJECT_ID=""
  local _OBJECT_TYPE=""

  echo -n "[" >"$_INSTALL_DIR/created-objects.json"
  awk '/"objectChanges":/,/],/' "$_INSTALL_DIR/publish-output.json" |
    while read -r line || [ -n "$line" ]; do
      # Increment _block_level when '{' is found anywhere in the line.
      if [[ $line == *"{"* ]]; then
        _block_level=$((_block_level + 1))
      fi
      # Decrement _block_level when '}' is found anywhere in the line.
      if [[ $line == *"}"* ]]; then
        _block_level=$((_block_level - 1))
        if [ $_block_level -eq 0 ]; then
          if [ "$_TYPE" = "created" ] && [ -n "$_OBJECT_TYPE" ] && [ -n "$_OBJECT_ID" ]; then
            if $_first_object_created; then
              _first_object_created=false
            else
              echo "," >>"$_INSTALL_DIR/created-objects.json"
            fi
            echo -n "{\"objectId\":\"$_OBJECT_ID\",\"type\":\"$_OBJECT_TYPE\"}" >>"$_INSTALL_DIR/created-objects.json"
          elif [ "$_TYPE" = "published" ] && [ -n "$_PACKAGE_ID" ]; then
            JSON_STR="[\"$_PACKAGE_ID\"]"
            echo "$JSON_STR" >"$_INSTALL_DIR/package-id.json"
          fi
          _TYPE=""
          _PACKAGE_ID=""
          _OBJECT_ID=""
          _OBJECT_TYPE=""
        fi
      fi
      # When _block_level=1, then extract the fields of interest.
      if [ $_block_level -eq 1 ]; then
        if [[ $line == *"\"type\":"* ]]; then
          _TYPE=$(echo "$line" | awk -F'"' '{print $4}')
        elif [[ $line == *"\"packageId\":"* ]]; then
          _PACKAGE_ID=$(echo "$line" | awk -F'"' '{print $4}')
        elif [[ $line == *"\"objectId\":"* ]]; then
          _OBJECT_ID=$(echo "$line" | awk -F'"' '{print $4}')
        elif [[ $line == *"\"objectType\":"* ]]; then
          _OBJECT_TYPE=$(echo "$line" | awk -F'"' '{print $4}')
        fi
      fi
    done

  echo "]" >>"$_INSTALL_DIR/created-objects.json"
}
export -f process_object_changes

has_unpublished_dependencies() {
  # Returns true if the "--with-unpublished-dependencies" option should be added.

  local _MOVE_TOML_DIR="$1"
  # For now, detect only Suibase specific local dependencies, might
  # allow this to work for any module later when  a more deterministic
  # way to manage sui dependencies exists...

  # Check in non-comment section for the following sub-string in order:
  # "=", "{", local", "=", "suibase/move/@suibase" and "}"
  sed 's/#.*//' "$_MOVE_TOML_DIR/Move.toml" | grep -q "=.*{.*local.*=.*suibase/move/@suibase.*}"
}
export -f has_unpublished_dependencies

# Returns success (0) for workdirs where the chain_id legitimately
# changes between publishes — currently only `localnet` (its genesis
# is regenerated by `localnet regen`). testnet/mainnet/devnet have
# stable chain_ids; any mismatch there is a real configuration error
# the user should resolve consciously, not a side-effect of auto-
# rewriting committed Move.toml on every publish.
should_auto_sync_chainid() {
  [ "$1" = "localnet" ]
}
export -f should_auto_sync_chainid

# Parse a JSON-RPC `sui_getChainIdentifier` response and emit the
# chain_id if (a) the envelope has a `"result"` field AND (b) the
# value is non-empty hex (no other characters). Anything else — JSON-
# RPC error envelopes, unrelated services answering on the proxy
# port, HTML 502 pages — yields empty. The caller treats empty as
# "skip the sync" rather than writing garbage into Move.toml.
extract_chain_id_from_response() {
  local _RESPONSE="$1"
  local _RESULT
  _RESULT=$(echo "$_RESPONSE" | sed -n 's/.*"result"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
  # Validate: non-empty hex string. Sui chain IDs today are 8-hex
  # chars (e.g. "9754208c") — be lenient about exact length so future
  # protocol changes don't surprise us, but reject anything that's
  # plainly not a chain identifier.
  if [ -n "$_RESULT" ] && printf '%s' "$_RESULT" | grep -Eq '^[0-9a-fA-F]+$'; then
    printf '%s' "$_RESULT"
  fi
}
export -f extract_chain_id_from_response

# Query the active workdir's chain identifier via JSON-RPC.
#
# Echoes the chain_id string on success, empty on any failure
# (proxy unreachable, proxy disabled, malformed response, garbage
# "result" value). Callers MUST treat empty as "skip the sync"
# rather than write whatever was returned.
get_current_chain_id() {
  # Defensive expansion: when proxy config is missing (e.g. a partial
  # suibase.yaml from before proxy_port_number was a required field,
  # or `proxy_enabled: false`), return empty rather than spamming
  # `parameter null or not set` to stderr or aborting subshells.
  local _HOST="${CFG_proxy_host_ip:-}"
  local _PORT="${CFG_proxy_port_number:-}"
  if [ -z "$_HOST" ] || [ -z "$_PORT" ]; then
    return 0
  fi
  local _RPC_URL="http://${_HOST}:${_PORT}"
  local _RESPONSE
  _RESPONSE=$(curl -sS --max-time 5 -X POST "$_RPC_URL" \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"sui_getChainIdentifier","params":[],"id":1}' 2>/dev/null)
  extract_chain_id_from_response "$_RESPONSE"
}
export -f get_current_chain_id

# Extract a single env's chain_id from a Move.toml [environments] table.
#
# Echoes the value on success, empty if the file/section/entry is
# missing. Reads only the [environments] section so an unrelated
# `<env_name> = "..."` line elsewhere can't be misread.
get_movetoml_env_chainid() {
  local _MOVE_TOML="$1"
  local _ENV_NAME="$2"
  [ -f "$_MOVE_TOML" ] || return 0
  awk -v env="$_ENV_NAME" '
    /^\[environments\]/ { in_section=1; next }
    /^\[/              { in_section=0 }
    in_section && $0 ~ "^[[:space:]]*"env"[[:space:]]*=" {
      # Pull the quoted value.
      if (match($0, /"[^"]*"/)) {
        v = substr($0, RSTART+1, RLENGTH-2)
        print v
        exit
      }
    }
  ' "$_MOVE_TOML"
}
export -f get_movetoml_env_chainid

# Rewrite a single env's chain_id in a Move.toml [environments] table.
#
# Idempotent: if the current value already matches `$3`, the file is
# NOT touched (mtime preserved). Emits a single line on stderr when a
# change is made so the publish output stays quiet on the common
# no-op case but is loud about the uncommon mutation case.
#
# Restricts the sed range to the [environments] block so a stray
# `<env_name> = "..."` line in another section (e.g. a dependency
# named after a network) can't be hit. The trailing `/^\[/` ends the
# range at the next TOML table header.
sync_movetoml_env_chainid() {
  local _MOVE_TOML="$1"
  local _ENV_NAME="$2"
  local _NEW_CHAIN_ID="$3"

  [ -f "$_MOVE_TOML" ] || return 0
  grep -q "^\[environments\]" "$_MOVE_TOML" || return 0

  local _CURRENT
  _CURRENT=$(get_movetoml_env_chainid "$_MOVE_TOML" "$_ENV_NAME")
  # Entry not present — nothing to sync; do not add (avoid surprises).
  [ -z "$_CURRENT" ] && return 0
  # Already matches — no-op (and don't touch mtime).
  [ "$_CURRENT" = "$_NEW_CHAIN_ID" ] && return 0

  # Mutate only within the [environments] block.
  sed -i.bak "/^\[environments\]/,/^\[/ s|^\([[:space:]]*${_ENV_NAME}[[:space:]]*=[[:space:]]*\"\)[^\"]*\(\"\)|\1${_NEW_CHAIN_ID}\2|" "$_MOVE_TOML"
  rm -f "${_MOVE_TOML}.bak"

  echo "  $_MOVE_TOML: $_ENV_NAME chain_id $_CURRENT -> $_NEW_CHAIN_ID" >&2
}
export -f sync_movetoml_env_chainid

# Sync chain_ids for both `<workdir>` and `<workdir>_proxy` env names
# in a single Move.toml. Both entries reference the same chain, so
# they share the same chain_id value.
sync_movetoml_workdir_chainids() {
  local _MOVE_TOML="$1"
  local _WORKDIR_NAME="$2"
  local _NEW_CHAIN_ID="$3"

  sync_movetoml_env_chainid "$_MOVE_TOML" "$_WORKDIR_NAME" "$_NEW_CHAIN_ID"
  sync_movetoml_env_chainid "$_MOVE_TOML" "${_WORKDIR_NAME}_proxy" "$_NEW_CHAIN_ID"
}
export -f sync_movetoml_workdir_chainids

# Extract every `local = "<path>"` entry from the [dependencies] AND
# [dev-dependencies] tables of a Move.toml. Prints one path per line.
#
# Implementation note: uses awk-with-gsub for portability across GNU
# awk, mawk, and BSD awk (macOS default). The earlier gawk-only 3-arg
# match() form was replaced because BSD awk silently aborted on it,
# and the sed fallback had a greedy-`.*` bug that captured only the
# LAST `local = "..."` on lines with nested inline tables.
extract_local_dep_paths() {
  local _MOVE_TOML="$1"
  [ -f "$_MOVE_TOML" ] || return 0
  awk '
    /^\[dependencies\]/     { in_section=1; next }
    /^\[dev-dependencies\]/ { in_section=1; next }
    /^\[/                   { in_section=0 }
    in_section {
      # Repeatedly strip the leftmost `local = "..."` match from the
      # line, emitting each captured path. This walks every occurrence
      # on one line (inline tables can nest `{ local = "a", x = { local = "b" } }`).
      s = $0
      while (match(s, /local[[:space:]]*=[[:space:]]*"[^"]*"/)) {
        chunk = substr(s, RSTART, RLENGTH)
        # chunk is now `local = "<path>"`; pull the quoted path out
        # by stripping prefix-up-to-quote and trailing quote.
        sub(/^[^"]*"/, "", chunk)
        sub(/"$/, "", chunk)
        if (chunk != "") print chunk
        s = substr(s, RSTART + RLENGTH)
      }
    }
  ' "$_MOVE_TOML"
}
export -f extract_local_dep_paths

# Walk all `local = "..."` deps in a Move.toml and sync each dep's
# Move.toml chain_ids, recursing through transitive deps. Includes
# both [dependencies] AND [dev-dependencies] so projects that put
# test-only Move packages under dev-deps don't see stale chain_ids
# breaking `localnet publish --test`.
#
# Cycle-safe via a visited-paths set so a bidirectional dep
# declaration (a → b → a) terminates.
#
# Skips paths that do not resolve to a Move.toml file.
sync_local_deps_chainids() {
  local _ROOT_MOVE_TOML="$1"
  local _WORKDIR_NAME="$2"
  local _NEW_CHAIN_ID="$3"

  # The visited set is a single $IFS-separated string of resolved
  # absolute paths. Pure-bash, no associative array needed (which
  # also keeps this portable to bash 3.2 on macOS).
  local _VISITED=""
  _sync_local_deps_chainids_recur "$_ROOT_MOVE_TOML" "$_WORKDIR_NAME" "$_NEW_CHAIN_ID" _VISITED
}
export -f sync_local_deps_chainids

# Internal recursive helper for sync_local_deps_chainids.
# $4 is the NAME of a caller-scoped variable holding the visited-set
# string; we mutate it through `eval` to thread state across the
# recursion without globals.
_sync_local_deps_chainids_recur() {
  local _MOVE_TOML="$1"
  local _WORKDIR_NAME="$2"
  local _NEW_CHAIN_ID="$3"
  local _VISITED_VAR="$4"

  [ -f "$_MOVE_TOML" ] || return 0
  local _ABS
  _ABS=$(cd "$(dirname "$_MOVE_TOML")" 2>/dev/null && pwd -P)/$(basename "$_MOVE_TOML")
  # Already visited? Stop.
  eval "case \" \${$_VISITED_VAR} \" in *\" \$_ABS \"*) return 0;; esac"
  eval "$_VISITED_VAR=\"\${$_VISITED_VAR} \$_ABS\""

  local _DIR
  _DIR=$(dirname "$_MOVE_TOML")

  local _DEP_PATH
  while IFS= read -r _DEP_PATH; do
    [ -z "$_DEP_PATH" ] && continue
    local _CANDIDATE="$_DIR/$_DEP_PATH/Move.toml"
    [ -f "$_CANDIDATE" ] || continue
    sync_movetoml_workdir_chainids "$_CANDIDATE" "$_WORKDIR_NAME" "$_NEW_CHAIN_ID"
    _sync_local_deps_chainids_recur "$_CANDIDATE" "$_WORKDIR_NAME" "$_NEW_CHAIN_ID" "$_VISITED_VAR"
  done < <(extract_local_dep_paths "$_MOVE_TOML")
}
export -f _sync_local_deps_chainids_recur

update_latest_symlinks() {
  # Following global variables must all be set:
  #   $PUBLISHED_DATA_DIR
  #   $MOVE_TOML_PACKAGE_NAME
  #   $WORKDIR_NAME
  #   $PACKAGE_UUID
  #   $PACKAGE_TIMESTAMP
  #
  # Will create the following symbolic links:
  #   $PUBLISHED_DATA_DIR/$MOVE_TOML_PACKAGE_NAME/most-recent -> $LINK_TARGET
  #   $PUBLISHED_DATA_DIR/$MOVE_TOML_PACKAGE_NAME/$PACKAGE_UUID/most-recent-timestamp -> $LINK_TARGET
  #
  #   where
  #      LINK_TARGET="$PUBLISHED_DATA_DIR/$MOVE_TOML_PACKAGE_NAME/$PACKAGE_UUID/$PACKAGE_TIMESTAMP"
  #
  # When the dev setup does not have multiple package with the *same name*, then it is sufficient
  # to use "most-recent".
  #
  # The $PACKAGE_UUID allow to differentiate when there are multiple packages with the same name.
  # The "most-recent-timestamp" within $PACKAGE_UUID dir can be used instead.
  #
  # The PACKAGE_UUID is the "uuid" field defined in the Suibase.yaml co-located with the Move.toml
  #
  # By default this UUID is generated for you. Alternatively, you can customize it if you prefer to
  # manage it yourself (you are responsible to keep it unique among all your projects!!!).
  #
  local _PACKAGE_ROOT_DIR="$PUBLISHED_DATA_DIR/$MOVE_TOML_PACKAGE_NAME"
  if [ ! -d "$_PACKAGE_ROOT_DIR" ]; then
    error_exit "Package directory not found: $_PACKAGE_ROOT_DIR"
  fi

  local _TARGET_UUID_DIR="$_PACKAGE_ROOT_DIR/$PACKAGE_UUID"
  if [ ! -d "$_TARGET_UUID_DIR" ]; then
    error_exit "Link target UUID directory not found: $_TARGET_UUID_DIR"
  fi

  local _LINK_TARGET_DIR="$_TARGET_UUID_DIR/$PACKAGE_TIMESTAMP"
  if [ ! -d "$_LINK_TARGET_DIR" ]; then
    error_exit "Link target timestamp not found: $_LINK_TARGET_DIR"
  fi

  local _LINK_FILEPATH="$_PACKAGE_ROOT_DIR/most-recent"
  local _TARGET_SYMLINK="./$PACKAGE_UUID/$PACKAGE_TIMESTAMP"
  if [ ! -L "$_LINK_FILEPATH" ]; then
    ln -s "$_TARGET_SYMLINK" "$_LINK_FILEPATH"
  else
    ln -nsf "$_TARGET_SYMLINK" "$_LINK_FILEPATH"
  fi

  _LINK_FILEPATH="$_PACKAGE_ROOT_DIR/$PACKAGE_UUID/most-recent-timestamp"
  _TARGET_SYMLINK="./$PACKAGE_TIMESTAMP"
  if [ ! -L "$_LINK_FILEPATH" ]; then
    ln -s "$_TARGET_SYMLINK" "$_LINK_FILEPATH"
  else
    ln -nsf "$_TARGET_SYMLINK" "$_LINK_FILEPATH"
  fi
}
export -f update_latest_symlinks
