#!/bin/bash

# Code that does the client.yaml and sui.keystore initialization for localnet.
#
# Also does only "regen" of the network when an existing client.yaml and sui.keystore
# already can be preserved (to re-use same client address).

# Intended to be sourced only in __workdir-exec.sh

apply_suibase_yaml_to_config_yaml() {
  local _GENDATA_DIR=$1

  # Detect coding error.
  if [ -z "${CFG_initial_fund_per_address:?}" ] || [ -z "${CFGDEFAULT_initial_fund_per_address:?}" ]; then
    setup_error "Bad suibase.yaml initial_fund_per_address [$CFG_initial_fund_per_address] [$CFGDEFAULT_initial_fund_per_address]"
  fi

  local _SEARCH_STRING="gas_value:.*\$"
  local _REPLACE_STRING="gas_value: $CFG_initial_fund_per_address"
  # echo "sed -i.bak -e \"s/$_SEARCH_STRING/$_REPLACE_STRING/g\" \"$_GENDATA_DIR/config.yaml\" && rm \"$_GENDATA_DIR/config.yaml.bak\""
  sed -i.bak -e "s/$_SEARCH_STRING/$_REPLACE_STRING/g" \
    "$_GENDATA_DIR/config.yaml" &&
    rm "$_GENDATA_DIR/config.yaml.bak"

  # Show the user when the changes in suibase.yaml was used.
  if [ "${CFG_initial_fund_per_address:?}" = "${CFGDEFAULT_initial_fund_per_address:?}" ]; then
    local _MSG="Applied default funding [$CFG_initial_fund_per_address]"
  else
    local _MSG="suibase.yaml: Applied initial_fund_per_address: [$CFG_initial_fund_per_address]"
  fi

  echo "$_MSG"

  # if enabled, do the same for the faucet funds (best effort)
  if [ "${CFG_sui_faucet_enabled:?}" = true ]; then
    if [ -z "$CFG_sui_faucet_genesis_funding" ] || [ -z "$CFGDEFAULT_sui_faucet_genesis_funding" ]; then
      echo "Warning: Bad suibase.yaml sui_faucet_genesis_funding [$CFG_sui_faucet_genesis_funding] [$CFGDEFAULT_sui_faucet_genesis_funding]"
      return
    fi
    if [ ! -f "$_GENDATA_DIR/faucet/faucet_wallet_address.txt" ]; then
      echo "Warning: Could not adjust faucet fund. Not supported by this version"
      return
    fi

    _SEARCH_STRING="gas_value:.*\$"
    _REPLACE_STRING="gas_value: $CFG_sui_faucet_genesis_funding"
    # Start on the 3rd character (skip the 0x of the faucet address)
    _TEXT_RANGE_START=$(cut -c 3- "$_GENDATA_DIR/faucet/faucet_wallet_address.txt")
    _TEXT_RANGE_END="gas_object_ranges"
    sed -i.bak -e "/$_TEXT_RANGE_START/,/$_TEXT_RANGE_END/ s/$_SEARCH_STRING/$_REPLACE_STRING/" \
      "$_GENDATA_DIR/config.yaml" &&
      rm "$_GENDATA_DIR/config.yaml.bak"

    # Show the user when the changes in suibase.yaml was used.
    if [ "$CFG_sui_faucet_genesis_funding" = "$CFGDEFAULT_sui_faucet_genesis_funding" ]; then
      local _MSG="Applied faucet default funding [$CFG_sui_faucet_genesis_funding]"
    else
      local _MSG="suibase.yaml: Applied sui_faucet_genesis_funding: [$CFG_sui_faucet_genesis_funding]"
    fi
    echo "$_MSG"
  fi
}

create_faucet_keystore() {
  local _SUI_BINARY=$1
  local _SRC_DIR=$2
  local _DEST_DIR=$3
  local _PUBKEY
  local _KEYPAIR

  rm -rf "$_DEST_DIR"
  mkdir -p "$_DEST_DIR"

  # Create a new "faucet" client/keystore at $_DESTDIR using
  # existing client/keystore at $_SRCDIR (for using the existing
  # client.yaml as template).

  # Create a sui.keystore with a single keypair
  (cd "$_DEST_DIR" && $SUI_BIN_ENV "$_SUI_BINARY" keytool generate ed25519 >&/dev/null)

  _PUBKEY_PATHNAME=$(ls "$_DEST_DIR"/*.key)
  _PUBKEY=$(basename "$_PUBKEY_PATHNAME" | sed 's/.key//g')
  if [ -z "$_PUBKEY" ]; then
    setup_error "Could not generate faucet key"
  fi
  _KEYPAIR=$(cat "$_PUBKEY_PATHNAME")
  if [ -z "$_KEYPAIR" ]; then
    setup_error "Could not generate faucet keypair"
  fi
  echo "[" >"$_DEST_DIR/sui.keystore"
  echo "\"$_KEYPAIR\"" >>"$_DEST_DIR/sui.keystore"
  echo "]" >>"$_DEST_DIR/sui.keystore"

  # Take a client known to be compatible as a template.
  \cp "$_SRC_DIR/client.yaml" "$_DEST_DIR"

  # Adjust the sui.keystore path
  sed -i.bak -e "s+$_SRC_DIR+$_DEST_DIR+g" "$_DEST_DIR/client.yaml" && rm "$_DEST_DIR/client.yaml.bak"

  # Just set the active address (don't care about the rest).
  $SUI_BIN_ENV "$_SUI_BINARY" client --client.config "$_DEST_DIR/client.yaml" switch --address "$_PUBKEY" >&/dev/null

  # Verify that this client.yaml/sui.keystore has
  # that keypair as the active-address.
  update_ACTIVE_ADDRESS_var "$_SUI_BINARY" "$_DEST_DIR/client.yaml"
  local _CHECK_ACTIVE=$ACTIVE_ADDRESS

  if [ "$_CHECK_ACTIVE" != "$_PUBKEY" ]; then
    setup_error "Could not set active the faucet key [$_PUBKEY], got [$_CHECK_ACTIVE]"
  fi

  # Create a handy file to find and parse for that single pubkey.
  echo "$_PUBKEY" >"$_DEST_DIR/faucet_wallet_address.txt"

  echo "New faucet keystore generated [$_PUBKEY]"
}

workdir_init_local() {
  # Two type of genesis:
  #  (1) Using "static" scripts/genesis_data when using sui-repo-default.
  #  (2) Using generated data after a set-sui-repo.
  #

  mkdir -p "$CONFIG_DATA_DIR_DEFAULT"
  cd_sui_log_dir

  if is_sui_repo_dir_default; then
    local _GENDATA_DIR="$GENERATED_GENESIS_DATA_DIR/default"
    rm -rf "$_GENDATA_DIR"
    mkdir -p "$_GENDATA_DIR"

    # Generate the templates to be used for building our own config.yaml
    mkdir -p "$_GENDATA_DIR/template"
    $SUI_BIN_ENV "$SUI_BIN_DIR/sui" genesis --working-dir "$_GENDATA_DIR/template" >&/dev/null
    $SUI_BIN_ENV "$SUI_BIN_DIR/sui" genesis --working-dir "$_GENDATA_DIR/template" --write-config "$_GENDATA_DIR/template/config.yaml" >&/dev/null
    # Get everything before the accounts section.
    sed '/accounts:/q' "$_GENDATA_DIR/template/config.yaml" >"$_GENDATA_DIR/config.yaml.template_head"
    # Check in case there is trailing stuff after the accounts section (for now it is empty).
    sed -n '/accounts:/,$p' "$_GENDATA_DIR/template/config.yaml" | sed '/^accounts/d' | sed -n '/^[a-z]/,$p' >"$_GENDATA_DIR/config.yaml.template_tail"
    rm -rf "$_GENDATA_DIR/template"
    # Find which static genesis_data version should be used.
    if version_greater_equal "$($SUI_BIN_ENV "$SUI_BIN_DIR/sui" -V)" "sui 0.31"; then
      _STATIC_SOURCE_DIR="$DEFAULT_GENESIS_DATA_DIR/0.31"
      # Use the templates to build the config.yaml.
      {
        cat "$_GENDATA_DIR/config.yaml.template_head"
        cat "$_STATIC_SOURCE_DIR/address.yaml.template"
        cat "$_GENDATA_DIR/config.yaml.template_tail"
      } >"$_GENDATA_DIR/config.yaml"
    else
      if version_greater_equal "$($SUI_BIN_ENV "$SUI_BIN_DIR/sui" -V)" "sui 0.28"; then
        _STATIC_SOURCE_DIR="$DEFAULT_GENESIS_DATA_DIR/0.28"
      else
        _STATIC_SOURCE_DIR="$DEFAULT_GENESIS_DATA_DIR/0.27"
      fi
      \cp -rf "$_STATIC_SOURCE_DIR/config.yaml" "$_GENDATA_DIR"
    fi

    \cp -rf "$_STATIC_SOURCE_DIR/client.yaml" "$_GENDATA_DIR"

    if [[ "${CFG_auto_key_generation:?}" == 'true' ]]; then
      \cp -rf "$_STATIC_SOURCE_DIR/sui.keystore" "$_GENDATA_DIR"
      \cp -rf "$_STATIC_SOURCE_DIR/recovery.txt" "$_GENDATA_DIR"
    else
      # Create an empty sui.keystore and clear the active-address in client.yaml.
      create_empty_keystore_file "$_GENDATA_DIR"
      # Replace everything after 'active_address: ' with a ~ in the file $_GENDATA_DIR/client.yaml
      clear_active_address_field "$_GENDATA_DIR/client.yaml"
    fi

    mkdir -p "$_GENDATA_DIR/faucet"
    \cp -rf "$_STATIC_SOURCE_DIR/faucet_sui.keystore" "$_GENDATA_DIR/faucet/sui.keystore"
    \cp -rf "$_STATIC_SOURCE_DIR/faucet_client.yaml" "$_GENDATA_DIR/faucet/client.yaml"
    \cp -rf "$_STATIC_SOURCE_DIR/faucet_wallet_address.txt" "$_GENDATA_DIR/faucet"
  else
    # This is the logic for when set-sui-repo
    local _GENDATA_DIR="$GENERATED_GENESIS_DATA_DIR/user-repo"
    if [ ! -d "$_GENDATA_DIR" ]; then
      mkdir -p "$_GENDATA_DIR"
      # Generate the genesis data for the very first time.
      $SUI_BIN_ENV "$SUI_BIN_DIR/sui" genesis --working-dir "$_GENDATA_DIR" >&/dev/null
      create_empty_keystore_file "$_GENDATA_DIR"
      add_test_addresses "$SUI_BIN_DIR/sui" "$_GENDATA_DIR/client.yaml" "$_GENDATA_DIR/recovery.txt"
      create_faucet_keystore "$SUI_BIN_DIR/sui" "$_GENDATA_DIR" "$_GENDATA_DIR/faucet"

      \cp "$_GENDATA_DIR/sui.keystore" "$_GENDATA_DIR/sui.keystore.temp_save"

      # Temporarly "merge" the keystores for generating the config.yaml
      # (so the faucet address can get prefunded as well)
      load_ACTIVE_KEYSTORE "$_GENDATA_DIR/sui.keystore"
      # Save a copy of ACTIVE_KEYSTORE into PREV_KEYSTORE
      PREV_KEYSTORE=("${ACTIVE_KEYSTORE[@]}")
      load_ACTIVE_KEYSTORE "$_GENDATA_DIR/faucet/sui.keystore"
      # Merge the keystores
      ACTIVE_KEYSTORE=("${PREV_KEYSTORE[@]}" "${ACTIVE_KEYSTORE[@]}")
      # Write the ACTIVE_KEYSTORE array.
      write_ACTIVE_KEYSTORE "$_GENDATA_DIR/sui.keystore"

      # Generate the config.yaml that will allow a deterministic setup.
      #$SUI_BIN_ENV "$SUI_BIN_DIR/sui" genesis --working-dir "$_GENDATA_DIR" --write-config "$_GENDATA_DIR/config.yaml" >& /dev/null
      $SUI_BIN_ENV "$SUI_BIN_DIR/sui" genesis --working-dir "$_GENDATA_DIR" --write-config "$_GENDATA_DIR/config.yaml"
      echo "New client addresses generated (new client.yaml and sui.keystore)"

      # Save this temporary keystore for potentially debugging.
      mv "$_GENDATA_DIR/sui.keystore" "$_GENDATA_DIR/sui.keystore.dbg_genesis"

      # Undo the merge (basically, remove the faucet address).
      mv "$_GENDATA_DIR/sui.keystore.temp_save" "$_GENDATA_DIR/sui.keystore"

      # Save an original of config.yaml for debugging (because it is modified on regen).
      \cp "$_GENDATA_DIR/config.yaml" "$_GENDATA_DIR/config.yaml.dbg_original"

      # Transform the client.yaml into templates (so we can easily replace any path into it later).
      _SEARCH_STRING="File:.*\$"
      _REPLACE_STRING="File: <PUT_CONFIG_DEFAULT_PATH_HERE>/sui.keystore"
      _TEXT_RANGE_START="keystore:"
      _TEXT_RANGE_END="envs:"
      sed -i.bak -e "/$_TEXT_RANGE_START/,/$_TEXT_RANGE_END/ s+$_SEARCH_STRING+$_REPLACE_STRING+g" \
        "$_GENDATA_DIR/client.yaml" &&
        rm "$_GENDATA_DIR/client.yaml.bak"

      _REPLACE_STRING="File: <PUT_FAUCET_PATH_HERE>/sui.keystore"
      sed -i.bak -e "/$_TEXT_RANGE_START/,/$_TEXT_RANGE_END/ s+$_SEARCH_STRING+$_REPLACE_STRING+g" \
        "$_GENDATA_DIR/faucet/client.yaml" &&
        rm "$_GENDATA_DIR/faucet/client.yaml.bak"

      echo "Genesis performed"
    fi
  fi

  apply_suibase_yaml_to_config_yaml "$_GENDATA_DIR"

  # Important NO OTHER files allowed in $_GENDATA_DIR prior to the genesis call, otherwise
  # it will fail!
  \cp -rf "$_GENDATA_DIR/sui.keystore" "$CONFIG_DATA_DIR_DEFAULT"
  \cp -rf "$_GENDATA_DIR/client.yaml" "$CONFIG_DATA_DIR_DEFAULT"

  # Replace a string in client.yaml to end up with an absolute path to the keystore.
  # Notice sed uses '+'' for seperator instead of '/' to avoid clash
  # with directory path. Also uses a .bak temp file because Mac (BSD) does not
  # allow in-place file change.
  sed -i.bak -e "s+<PUT_CONFIG_DEFAULT_PATH_HERE>+$CONFIG_DATA_DIR_DEFAULT+g" "$CONFIG_DATA_DIR_DEFAULT/client.yaml" && rm "$CONFIG_DATA_DIR_DEFAULT/client.yaml.bak"

  # "regen" from the genesis config.yaml
  if [ "$DEBUG_PARAM" = true ]; then
    $SUI_BIN_ENV "$SUI_BIN_DIR/sui" genesis --from-config "$_GENDATA_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT"
  else
    $SUI_BIN_ENV "$SUI_BIN_DIR/sui" genesis --from-config "$_GENDATA_DIR/config.yaml" --working-dir "$CONFIG_DATA_DIR_DEFAULT" >&/dev/null
  fi

  # Now is a safe time to add more files to $_GENDATA_DIR
  if [ -f "$_GENDATA_DIR/recovery.txt" ]; then
    \cp -rf "$_GENDATA_DIR/recovery.txt" "$CONFIG_DATA_DIR_DEFAULT"
  fi

  # Adjust the sui.keystore and client.yaml from commands in the suibase.yaml
  copy_private_keys_yaml_to_keystore "$CONFIG_DATA_DIR_DEFAULT/sui.keystore"

  # Update the client.yaml active address field if not set and at least one address is available.
  update_client_yaml_active_address

  # Install the faucet config.
  rm -rf "$WORKDIRS/$WORKDIR/faucet"
  mkdir -p "$WORKDIRS/$WORKDIR/faucet"
  \cp -rf "$_GENDATA_DIR/faucet/sui.keystore" "$WORKDIRS/$WORKDIR/faucet"
  \cp -rf "$_GENDATA_DIR/faucet/client.yaml" "$WORKDIRS/$WORKDIR/faucet"
  # Adjust the sui.keystore path
  sed -i.bak -e "s+<PUT_FAUCET_PATH_HERE>+$WORKDIRS/$WORKDIR/faucet+g" "$WORKDIRS/$WORKDIR/faucet/client.yaml" && rm "$WORKDIRS/$WORKDIR/faucet/client.yaml.bak"

  # When need to start in foreground to debug.
  if [ "$DEBUG_PARAM" = true ]; then
    echo "Starting localnet process (foreground for debug)"
    $SUI_BIN_ENV "$SUI_BIN_DIR/sui" start --network.config "$NETWORK_CONFIG"
    exit
  fi
}
export -f workdir_init_local
