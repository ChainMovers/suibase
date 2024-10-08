# shellcheck shell=bash

# Intended to be sourced only in __workdir-exec.sh

# Code that does the client.yaml and sui.keystore initialization for
# remote networks (devnet/testnet).

# Uses an existing client.yaml and sui.keystore if already installed by the user.

workdir_init_remote() {

  mkdir -p "$CONFIG_DATA_DIR_DEFAULT"
  # Deprecated cd_sui_log_dir

  # The config-default/client.yaml should already be here from the templates, but
  # do the following logic to "repair" in case someone setup is mess up (they can
  # delete the config-default directory and let it rebuild to default).
  if [ ! -f "$CONFIG_DATA_DIR_DEFAULT/client.yaml" ]; then
    # Attempt to copy from templates.
    SRC="$SCRIPTS_DIR/templates/$WORKDIR/config-default/client.yaml"
    if [ -f "$SRC" ]; then
      \cp "$SRC" "$CONFIG_DATA_DIR_DEFAULT/client.yaml"
    fi

    # Final check.
    if [ ! -f "$CONFIG_DATA_DIR_DEFAULT/client.yaml" ]; then
      setup_error "Missing [$CONFIG_DATA_DIR_DEFAULT/client.yaml]"
    fi
  fi

  # Check to replace the placeholder from the template file.
  SEARCH_STRING="<PUT_WORKING_DIR_PATH_HERE>"
  STR_FOUND=$(grep "$SEARCH_STRING" "$CLIENT_CONFIG")
  if [ -n "$STR_FOUND" ]; then
    # Replace a string in client.yaml to end up with an absolute path to the keystore.
    # Notice sed uses '+'' for seperator instead of '/' to avoid clash
    # with directory path. Also uses a .bak temp file because Mac (BSD) does not
    # allow in-place file change.
    REPLACE_STRING="$WORKDIRS/$WORKDIR"
    sed -i.bak -e "s+$SEARCH_STRING+$REPLACE_STRING+g" "$CONFIG_DATA_DIR_DEFAULT/client.yaml" && rm "$CONFIG_DATA_DIR_DEFAULT/client.yaml.bak"
  fi

  start_all_services

  # Create client addresses, but only if there is no sui.keystore already (and allowed by suibase.yaml)
  if [ ! -f "$CONFIG_DATA_DIR_DEFAULT/sui.keystore" ]; then
    add_test_addresses "$SUI_BIN_DIR/sui" "$CONFIG_DATA_DIR_DEFAULT/client.yaml" "$CONFIG_DATA_DIR_DEFAULT/recovery.txt"
    if [ ! -f "$CONFIG_DATA_DIR_DEFAULT/sui.keystore" ]; then
      create_empty_keystore_file "$CONFIG_DATA_DIR_DEFAULT"
    fi
  fi

  # Allow user to custom insert its own private keys.
  copy_private_keys_yaml_to_keystore "$CONFIG_DATA_DIR_DEFAULT/sui.keystore"

  # Update the client.yaml active address field if not set and at least one address is available.
  update_client_yaml_active_address
}
