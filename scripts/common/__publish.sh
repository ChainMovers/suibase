#!/bin/bash

# Code that does publish modules to a Sui network

# Intended to be sourced only in __workdir-exec.sh

publish_all() {

  local _PASSTHRU_OPTIONS="${*}"

  if [ -z "$MOVE_TOML_PACKAGE_NAME" ]; then
    echo "Package name could not be found"
    exit
  fi

  # Do a pre publication handshake with the suibase-daemon.
  # On success, will get the global PACKAGE_UUID variable set.
  # On failure, the script will exit_error.
  do_suibase_daemon_pre_publish "$MOVE_TOML_DIR" "$MOVE_TOML_PACKAGE_NAME"

  echo "Package name=[$MOVE_TOML_PACKAGE_NAME]"

  local _SUB_INSTALL_DIR="$MOVE_TOML_PACKAGE_NAME/$PACKAGE_UUID/$PACKAGE_TIMESTAMP"
  echo "Suibase output: ~/suibase/workdirs/$WORKDIR_NAME/published-data/$_SUB_INSTALL_DIR"

  INSTALL_DIR="$PUBLISHED_DATA_DIR/$_SUB_INSTALL_DIR"

  mkdir -p "$INSTALL_DIR"

  publish_clear_output "$INSTALL_DIR"

  sync_client_yaml

  # Build the Move package for publication.
  echo "Will publish using the sui client matching the network. Command line is:"
  CMD="$SUI_EXEC client publish --gas-budget 20000000 --install-dir \"$INSTALL_DIR\" \"$MOVE_TOML_DIR\" $_PASSTHRU_OPTIONS --json 2>&1 1>$INSTALL_DIR/publish-output.json"

  echo "$CMD"
  # Execute $CMD
  eval "$CMD"

  #  TODO Investigate problem with exit status here...

  # Create the created_objects.json file.
  echo -n "[" >"$INSTALL_DIR/created-objects.json"
  local _first_object_created=true
  # Get all the objectid
  awk '/"created":/,/],/' "$INSTALL_DIR/publish-output.json" |
    grep objectId | sed 's/\"//g; s/,//g' | tr -d "[:blank:]" |
    while read -r line; do
      # Extract first hexadecimal literal found.
      # Define the seperator (IFS) as the JSON ':'
      local _ID=""
      IFS=":"
      for _i in $line; do
        if beginswith 0x "$_i"; then
          _ID=$_i
          break
        fi
      done
      # Best-practice to revert IFS to default.
      unset IFS
      #echo "$_ID"
      if [ -n "$_ID" ]; then
        # Get the type of the object
        object_type=$($SUI_EXEC client object "$_ID" --json | grep "type" | sed 's/,//g' | tr -d "[:blank:]" | head -n 1)
        if [ -z "$object_type" ]; then
          # To be removed eventually. Version 0.27 devnet was working differently.
          object_type=$($SUI_EXEC client object "$_ID" --json | grep "dataType" | grep "package")
          if [ -n "$object_type" ]; then
            _found_id=true
          fi
        else
          if $_first_object_created; then
            _first_object_created=false
          else
            echo "," >>"$INSTALL_DIR/created-objects.json"
          fi

          echo -n "{\"objectid\":\"$_ID\",$object_type}" >>"$INSTALL_DIR/created-objects.json"
          #echo "ot=[$object_type]"
          if [ "$object_type" = "\"type\":\"package\"" ]; then
            _found_id=true
          fi
        fi

        if $_found_id; then
          JSON_STR="[\"$_ID\"]"
          echo "$JSON_STR" >"$INSTALL_DIR/package-id.json"
          _found_id=false
        fi

      fi
    done
  echo "]" >>"$INSTALL_DIR/created-objects.json"

  # Load back the package-id.json from the file for validation
  local _ID_PACKAGE
  if [ -f "$INSTALL_DIR/package-id.json" ]; then
    _ID_PACKAGE=$(sed 's/\[//g; s/\]//g; s/"//g;' "$INSTALL_DIR/package-id.json")
  fi

  # echo "Package ID=[$_ID_PACKAGE]"

  if [ -z "$_ID_PACKAGE" ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "suibase: Publication failed."
  fi

  # Test the publication by retreiving object information from the network
  # using that parsed package id.
  echo "suibase: Verifying new package is on network..."
  validation=$($SUI_EXEC client object "$_ID_PACKAGE" | grep -i "package")
  if [ -z "$validation" ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "suibase: Unexpected object type (Not a package)"
  fi
  JSON_STR="[\"$_ID_PACKAGE\"]"
  echo "$JSON_STR" >"$INSTALL_DIR/package-id.json"

  # Update the 'latest' symlink.
  update_latest_symlinks

  echo "Package ID is $JSON_STR"
  echo "Also written in [~/suibase/workdirs/$WORKDIR_NAME/published-data/$MOVE_TOML_PACKAGE_NAME/most-recent/package-id.json]"
  echo "Publication Successful"

  # Push new information to suibase-daemon.
  do_suibase_daemon_post_publish "$MOVE_TOML_DIR" "$MOVE_TOML_PACKAGE_NAME" "$PACKAGE_UUID" "$PACKAGE_TIMESTAMP" "$_ID_PACKAGE"
}
export -f publish_all

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
