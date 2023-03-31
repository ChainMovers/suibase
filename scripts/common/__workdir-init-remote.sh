#!/bin/bash

# Code that does the client.yaml and sui.keystore initialization for
# remote networks (devnet/testnet).

# Uses an existing client.yaml and sui.keystore if already installed by the user.

# Intended to be sourced only in __workdir-exec.sh

workdir_init_remote() {

    mkdir -p "$CONFIG_DATA_DIR_DEFAULT"
    cd_sui_log_dir;

    # The config-default/client.yaml should already be here from the templates, but
    # do the following logic to "repair" in case someone setup is mess up (they can
    # delete the config-default directory and let it rebuild to default).
    if [ ! -f "$CONFIG_DATA_DIR_DEFAULT/client.yaml" ]; then
      # Attempt to copy from templates.
      SRC="$SCRIPTS_DIR/templates/$WORKDIR/config-default/client.yaml"
      if [ -f "$SRC" ]; then
        cp "$SRC" "$CONFIG_DATA_DIR_DEFAULT/client.yaml"
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

    # Create client addresses, but only if there is no sui.keystore already.

    # TODO put some sui-base.yaml customization here! and some error handling!
    if [ ! -f "$CONFIG_DATA_DIR_DEFAULT/sui.keystore" ]; then
      add_test_addresses "$SUI_BIN_DIR/sui" "$CONFIG_DATA_DIR_DEFAULT/client.yaml" "$CONFIG_DATA_DIR_DEFAULT/recovery.txt"
    fi

    STR_FOUND=$(grep "active_address:" "$CLIENT_CONFIG" | grep "~")
    if [ -n "$STR_FOUND" ]; then
      # The following trick will update the client.yaml active address field if not set!
      # (a client call switch to an address, using output of another client call picking a default).
      $SUI_EXEC client switch --address $($SUI_EXEC client active-address)
    fi
}
