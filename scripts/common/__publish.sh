#!/bin/bash

# Code that does publish modules to a Sui network

# Intended to be sourced only in __workdir-exec.sh

publish_all() {

  local _PASSTHRU_OPTIONS="${*}"

  if [ -z "$MOVE_TOML_PACKAGE_NAME" ]; then
    echo "suibase: Package name could not be found"
    exit
  fi

  # Add default --gas-budget if not specified.
  if ! has_param "" "--gas-budget" $_PASSTHRU_OPTIONS; then
    _PASSTHRU_OPTIONS="$_PASSTHRU_OPTIONS --gas-budget 500000000"
  fi

  # Add --json, but only if not already specified by the caller.
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
  echo "==================== Explorer Links ======================"
  echo "Package [https://suiexplorer.com/object/$_ID_PACKAGE_FOR_LINK?network=$_WORKDIR_NAME_FOR_LINK]"
  if [ -n "$SUI_PUBLISH_TXDIGEST" ]; then
    echo "TxBlock [https://suiexplorer.com/txblock/$SUI_PUBLISH_TXDIGEST?network=$_WORKDIR_NAME_FOR_LINK]"
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
