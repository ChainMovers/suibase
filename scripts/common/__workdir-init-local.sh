# Code that does the client.yaml and sui.keystore initialization for localnet.
#
# Also does only "regen" of the network when an existing client.yaml and sui.keystore
# already can be preserved (to re-use same client address).

# Intended to be sourced only in __workdir-exec.sh

workdir_init_local() {
    # Two type of genesis:
    #  (1) Using "static" scripts/genesis_data when using sui-repo-default.
    #  (2) Using generated data after a set-sui-repo.
    #

    mkdir -p "$CONFIG_DATA_DIR_DEFAULT"

    if is_sui_repo_dir_default; then
      # Find which static genesis_data version should be used.
      # Only two so far >=0.28 and everything else below.
      if version_greater_equal "$($SUI_BIN_DIR/sui -V)" "sui 0.28"; then
        STATIC_SOURCE_DIR="$DEFAULT_GENESIS_DATA_DIR/0.28"
      else
        STATIC_SOURCE_DIR="$DEFAULT_GENESIS_DATA_DIR/0.27"
      fi

      if [ "$DEBUG_RUN" = true ]; then
        $SUI_BIN_DIR/sui genesis --from-config "$STATIC_SOURCE_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT"
      else
        $SUI_BIN_DIR/sui genesis --from-config "$STATIC_SOURCE_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT" >& /dev/null
      fi

      yes | cp -rf "$STATIC_SOURCE_DIR/sui.keystore" "$CONFIG_DATA_DIR_DEFAULT"
      yes | cp -rf "$STATIC_SOURCE_DIR/client.yaml" "$CONFIG_DATA_DIR_DEFAULT"

      SEARCH_STRING="<PUT_WORKING_DIR_PATH_HERE>"
      REPLACE_STRING="$CONFIG_DATA_DIR_DEFAULT"
    else
      # This is the logic for when set-sui-repo
      if [ ! -d "$GENERATED_GENESIS_DATA_DIR" ]; then
        mkdir -p "$GENERATED_GENESIS_DATA_DIR"
        # Generate the genesis data for the very first time.
        $SUI_BIN_DIR/sui genesis --working-dir "$GENERATED_GENESIS_DATA_DIR" >& /dev/null
        # Generate the config.yaml that will allow a deterministic setup.
        $SUI_BIN_DIR/sui genesis --working-dir "$GENERATED_GENESIS_DATA_DIR" --write-config "$GENERATED_GENESIS_DATA_DIR/config.yaml" >& /dev/null
        echo "Genesis performed. New client addresses generated (new client.yaml and sui.keystore)"
      fi

      # "regen" from the genesis config.yaml
      if [ "$DEBUG_RUN" = true ]; then
        $SUI_BIN_DIR/sui genesis --from-config "$GENERATED_GENESIS_DATA_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT"
      else
        $SUI_BIN_DIR/sui genesis --from-config "$GENERATED_GENESIS_DATA_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT" >& /dev/null
      fi

      yes | cp -rf "$GENERATED_GENESIS_DATA_DIR/sui.keystore" "$CONFIG_DATA_DIR_DEFAULT"
      yes | cp -rf "$GENERATED_GENESIS_DATA_DIR/client.yaml" "$CONFIG_DATA_DIR_DEFAULT"

      SEARCH_STRING="genesis-data"
      REPLACE_STRING="config"
    fi

    # Replace a string in client.yaml to end up with an absolute path to the keystore.
    # Notice sed uses '+'' for seperator instead of '/' to avoid clash
    # with directory path. Also uses a .bak temp file because Mac (BSD) does not
    # allow in-place file change.
    sed -i.bak -e "s+$SEARCH_STRING+$REPLACE_STRING+g" "$CONFIG_DATA_DIR_DEFAULT/client.yaml" && rm "$CONFIG_DATA_DIR_DEFAULT/client.yaml.bak"

    # When need to start in foreground to debug.
    if [ "$DEBUG_RUN" = true ]; then
      echo "Starting localnet process (foreground for debug)"
      $SUI_BIN_DIR/sui start --network.config "$NETWORK_CONFIG"
      exit
    fi
}
