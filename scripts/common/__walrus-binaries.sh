# shellcheck shell=bash

# You must source __globals.sh and __apps.sh before __walrus-binaries.sh

update_walrus_config() {
  local _WORKDIR="$1"
  local _CONFIG_NAME="$2"

  # Do nothing if not testnet/mainnet workdirs
  if [ "$_WORKDIR" != "testnet" ] && [ "$_WORKDIR" != "mainnet" ]; then
    return 0
  fi

  # Do nothing if the workdir/config-default does not exists (something getting done out-of-order?).
  if [ ! -d "$WORKDIRS/$_WORKDIR/config-default" ]; then
    return 0
  fi

  # Copy ~/suibase/scripts/templates/$WORKDIR/config-default/_CONFIG_NAME
  # to $WORKDIRS/$WORKDIR/config/_CONFIG_NAME if:
  #   - it does not exists.
  #   - Any line with an "0x" is different.
  #
  # Note: On copy replace $HOME with the actual home directory.
  local _TEMPLATE_PATH="$SUIBASE_DIR/scripts/templates/$_WORKDIR/config-default/$_CONFIG_NAME"
  local _DEST_PATH="$WORKDIRS/$_WORKDIR/config-default/$_CONFIG_NAME"
  local _DO_COPY=false

  if [ -f "$_DEST_PATH" ]; then
      local _TEMPLATE_CONTENT
    _TEMPLATE_CONTENT=$(cat "$_TEMPLATE_PATH")
    _TEMPLATE_CONTENT=$(echo "$_TEMPLATE_CONTENT" | sed "s|\\\$HOME/|$HOME/|g")

    # Extract and compare only lines containing "0x" from both files
    local _USER_0X_LINES
    local _TEMPLATE_0X_LINES

    _USER_0X_LINES=$(grep "0x" "$_DEST_PATH" 2>/dev/null || echo "")
    _TEMPLATE_0X_LINES=$(echo "$_TEMPLATE_CONTENT" | grep "0x" 2>/dev/null || echo "")

    # Check if the "0x" lines are different
    if [ "$_USER_0X_LINES" != "$_TEMPLATE_0X_LINES" ]; then
      _DO_COPY=true
    fi
  else
    _DO_COPY=true
  fi

  if [ "$_DO_COPY" = "true" ]; then
    mkdir -p "$WORKDIRS/$_WORKDIR/config-default"

    cat "$_TEMPLATE_PATH" | sed "s|\\\$HOME/|$HOME/|g" > "$_DEST_PATH"

    echo "$_WORKDIR/$_CONFIG_NAME updated with defaults."
  fi
}
export -f update_walrus_config

update_walrus_configs() {
  local _WORKDIR="$1"

  update_walrus_config "$_WORKDIR" "client_config.yaml"
  update_walrus_config "$_WORKDIR" "sites-config.yaml"
}
export -f update_walrus_configs

update_walrus_app() {
  local _WORKDIR="$1"
  local app_obj="$2" # "walrus" or "site_builder"

  init_app_obj "$app_obj" "$_WORKDIR"
  app_call "$app_obj" "set_local_vars"

  get_app_var "$app_obj" "assets_name"
  local _ASSETS_NAME=$APP_VAR

  get_app_var "$app_obj" "is_installed"
  local _IS_INSTALLED=$APP_VAR

  if [ "$_IS_INSTALLED" != "true" ]; then
    # Note: cli_mutex_lock remains re-entrant within the same process.
    #       Use the same "walrus" mutex for all walrus app installations.
    cli_mutex_lock "walrus"

    get_app_var "$app_obj" "local_bin_version"
    local _OLD_VERSION=$APP_VAR

    app_call "$app_obj" "install"
    app_call "$app_obj" "set_local_vars"
    get_app_var "$app_obj" "is_installed"
    _IS_INSTALLED=$APP_VAR
    get_app_var "$app_obj" "local_bin_version"
    local _NEW_VERSION=$APP_VAR

    if [ "$_IS_INSTALLED" != "true" ] || [ -z "$_NEW_VERSION" ]; then
      setup_error "Failed to install $_ASSETS_NAME"
    fi

    if [ "$_OLD_VERSION" != "$_NEW_VERSION" ]; then
      if [ -n "$_OLD_VERSION" ]; then
        echo "$_ASSETS_NAME upgraded from $_OLD_VERSION to $_NEW_VERSION"
        _WAS_UPGRADED=true
      else
        echo "$_ASSETS_NAME $_NEW_VERSION installed"
      fi
    fi
  fi

  #app_call "$app_obj" "print"
}
export -f update_walrus_app

update_walrus() {

  local _WORKDIR="$1"

  # Return 0 on success or not needed.

  # Do nothing if not testnet/mainnet workdirs
  if [ "$_WORKDIR" != "testnet" ] && [ "$_WORKDIR" != "mainnet" ]; then
    return 0
  fi

  # Do nothing if the workdir does not exists (something getting done out-of-order?).
  if [ ! -d "$WORKDIRS/$_WORKDIR" ]; then
    return 0
  fi

  update_walrus_app "$_WORKDIR" "walrus"
  update_walrus_app "$_WORKDIR" "site_builder"
  update_walrus_configs "$_WORKDIR"

  return 0
}
export -f update_walrus

exit_if_walrus_binary_not_ok() {
  # This is for common "operator" error (not doing command in right order).
  if [ ! -f "$WALRUS_BIN_DIR/walrus" ]; then
    echo
    echo "The walrus binary for $WORKDIR was not found."
    echo
    echo " Do one of the following to install it:"
    echo "    $WORKDIR start"
    echo "    $WORKDIR update"
    echo
    exit 1
  fi

  if [ ! -f "$WALRUS_BIN_DIR/site-builder" ]; then
    echo
    echo "The site-builder binary for $WORKDIR was not found."
    echo
    echo " Do one of the following to install it:"
    echo "    $WORKDIR start"
    echo "    $WORKDIR update"
    echo
    exit 1
  fi

}
export -f exit_if_walrus_binary_not_ok

is_walrus_binary_ok() {
  # Keep this one match the logic of exit_if_walrus_binary_not_ok()
  # The difference is this function should NEVER exit because it
  # is used to detect problems and have the caller try to repair the
  # binary.
  if [ ! -f "$WALRUS_BIN_DIR/walrus" ] || [ ! -f "$SITE_BUILDER_BIN_DIR/site-builder" ]; then
    false
    return
  fi

  # Get the versions, but in a way that would not exit on failure.
  local __VERSION_ATTEMPT
  _VERSION_ATTEMPT=$("$WALRUS_BIN_DIR/walrus" --version)

  # TODO Should parse to check that a version is indeed returned...
  if [ -z "$_VERSION_ATTEMPT" ]; then
    false
    return
  fi

  _VERSION_ATTEMPT=$("$WALRUS_BIN_DIR/site-builder" --version)
  if [ -z "$_VERSION_ATTEMPT" ]; then
    false
    return
  fi

  true
  return
}
export -f is_walrus_binary_ok
