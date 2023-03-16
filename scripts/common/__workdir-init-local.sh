#!/bin/bash

# Code that does the client.yaml and sui.keystore initialization for localnet.
#
# Also does only "regen" of the network when an existing client.yaml and sui.keystore
# already can be preserved (to re-use same client address).

# Intended to be sourced only in __workdir-exec.sh

apply_sui_base_yaml_to_config_yaml() {
  local _GENDATA_DIR=$1
  local _CUR_VALUE
  # Detect coding error.
  if [ -z "$CFG_initial_fund_per_address" ] || [ -z "$CFGDEFAULT_initial_fund_per_address" ]; then
    setup_error "Missing sui-base.yaml initial_fund_per_address [$CFG_initial_fund_per_address] [$CFGDEFAULT_initial_fund_per_address]"
  fi

  _CUR_VALUE=$(grep " gas_value:" "$_GENDATA_DIR/config.yaml" | head -1 | tr -cd '[:digit:]')

  if [ "$_CUR_VALUE" != "$CFG_initial_fund_per_address" ]; then
    local _SEARCH_STRING="gas_value:.*\$"
    local _REPLACE_STRING="gas_value: $CFG_initial_fund_per_address"
    # echo "sed -i.bak -e \"s/$_SEARCH_STRING/$_REPLACE_STRING/g\" \"$_GENDATA_DIR/config.yaml\" && rm \"$_GENDATA_DIR/config.yaml.bak\""
    sed -i.bak -e "s/$_SEARCH_STRING/$_REPLACE_STRING/g" "$_GENDATA_DIR/config.yaml" && rm "$_GENDATA_DIR/config.yaml.bak"
  fi

  # Show the user when the changes in sui-base.yaml was used.
  if [ "$CFG_initial_fund_per_address" = "$CFGDEFAULT_initial_fund_per_address" ]; then
    local _MSG="Applied default funding [$CFG_initial_fund_per_address]"
  else
    local _MSG="sui-base.yaml: Applied initial_fund_per_address: [$CFG_initial_fund_per_address]"
  fi
  echo "$_MSG"
}

adjust_default_keystore() {
  # Add a few more addresses to the default sui.keystore
  local _SUI_BINARY=$1
  local _CLIENT_FILE=$2
  for _ in {1..5}; do
  #  _SUI_BINARY client --client.config $_CLIENT_FILE new-address ed25519 >& /dev/null
    $_SUI_BINARY client --client.config "$_CLIENT_FILE" new-address secp256k1 >& /dev/null
    $_SUI_BINARY client --client.config "$_CLIENT_FILE" new-address secp256r1 >& /dev/null
  done
}

workdir_init_local() {
    # Two type of genesis:
    #  (1) Using "static" scripts/genesis_data when using sui-repo-default.
    #  (2) Using generated data after a set-sui-repo.
    #

    mkdir -p "$CONFIG_DATA_DIR_DEFAULT"

    if is_sui_repo_dir_default; then
      local _GENDATA_DIR="$GENERATED_GENESIS_DATA_DIR/default"
      rm -rf "$_GENDATA_DIR"
      mkdir -p "$_GENDATA_DIR"
      # Find which static genesis_data version should be used.
      # Only two so far >=0.28 and everything else below.
      if version_greater_equal "$("$SUI_BIN_DIR/sui" -V)" "sui 0.28"; then
        _STATIC_SOURCE_DIR="$DEFAULT_GENESIS_DATA_DIR/0.28"
      else
        _STATIC_SOURCE_DIR="$DEFAULT_GENESIS_DATA_DIR/0.27"
      fi
      yes | cp -rf "$_STATIC_SOURCE_DIR/sui.keystore" "$_GENDATA_DIR"
      yes | cp -rf "$_STATIC_SOURCE_DIR/client.yaml" "$_GENDATA_DIR"
      yes | cp -rf "$_STATIC_SOURCE_DIR/config.yaml" "$_GENDATA_DIR"

      local _SEARCH_STRING="<PUT_WORKING_DIR_PATH_HERE>"
      local _REPLACE_STRING="$CONFIG_DATA_DIR_DEFAULT"
    else
      # This is the logic for when set-sui-repo
      local _GENDATA_DIR="$GENERATED_GENESIS_DATA_DIR/user-repo"
      if [ ! -d "$_GENDATA_DIR" ]; then
        mkdir -p "$_GENDATA_DIR"
        # Generate the genesis data for the very first time.
        "$SUI_BIN_DIR/sui" genesis --working-dir "$_GENDATA_DIR" >& /dev/null
        adjust_default_keystore "$SUI_BIN_DIR/sui" "$_GENDATA_DIR/client.yaml"

        # Generate the config.yaml that will allow a deterministic setup.
        "$SUI_BIN_DIR/sui" genesis --working-dir "$_GENDATA_DIR" --write-config "$_GENDATA_DIR/config.yaml" >& /dev/null
        echo "Genesis performed. New client addresses generated (new client.yaml and sui.keystore)"
      fi

      local _SEARCH_STRING="genesis-data/user-repo"
      local _REPLACE_STRING="config"
    fi

    apply_sui_base_yaml_to_config_yaml "$_GENDATA_DIR";

    yes | cp -rf "$_GENDATA_DIR/sui.keystore" "$CONFIG_DATA_DIR_DEFAULT"
    yes | cp -rf "$_GENDATA_DIR/client.yaml" "$CONFIG_DATA_DIR_DEFAULT"

    # Replace a string in client.yaml to end up with an absolute path to the keystore.
    # Notice sed uses '+'' for seperator instead of '/' to avoid clash
    # with directory path. Also uses a .bak temp file because Mac (BSD) does not
    # allow in-place file change.
    sed -i.bak -e "s+$_SEARCH_STRING+$_REPLACE_STRING+g" "$CONFIG_DATA_DIR_DEFAULT/client.yaml" && rm "$CONFIG_DATA_DIR_DEFAULT/client.yaml.bak"

    # "regen" from the genesis config.yaml
    if [ "$DEBUG_RUN" = true ]; then
      "$SUI_BIN_DIR/sui" genesis --from-config "$_GENDATA_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT"
    else
      "$SUI_BIN_DIR/sui" genesis --from-config "$_GENDATA_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT" >& /dev/null
    fi

    # When need to start in foreground to debug.
    if [ "$DEBUG_RUN" = true ]; then
      echo "Starting localnet process (foreground for debug)"
      "$SUI_BIN_DIR/sui" start --network.config "$NETWORK_CONFIG"
      exit
    fi
}
