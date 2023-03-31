#!/bin/bash

# Do not call this script directly. It is a "common script" sourced by other sui-base scripts.
#
# It initializes a bunch of environment variable, verify that some initialization took
# place, identify some common user errors etc...

USER_CWD=$(pwd -P)

SUI_BASE_VERSION="0.1.0"

# Sui-base does not work with version below these.
MIN_SUI_VERSION="sui 0.27.0"
MIN_RUST_VERSION="rustc 1.65.0"

# Mandatory command line:
#    $1 : Should be the "$0" of the caller script.
#    $2 : Should be the workdir string (e.g. "active", "localnet"... )
SCRIPT_PATH="$(dirname "$1")"
SCRIPT_NAME="$(basename "$1")"
WORKDIR="$2"

# Two key directories location.
SUI_BASE_DIR="$HOME/sui-base"
WORKDIRS="$SUI_BASE_DIR/workdirs"

# Some other commonly used locations.
LOCAL_BIN="$HOME/.local/bin"
SCRIPTS_DIR="$SUI_BASE_DIR/scripts"
SUI_REPO_DIR="$WORKDIRS/$WORKDIR/sui-repo"
CONFIG_DATA_DIR="$WORKDIRS/$WORKDIR/config"
PUBLISHED_DATA_DIR="$WORKDIRS/$WORKDIR/published-data"
FAUCET_DIR="$WORKDIRS/$WORKDIR/faucet"
SUI_BIN_DIR="$SUI_REPO_DIR/target/debug"

case $WORKDIR in
  localnet)
    SUI_SCRIPT="lsui"
    ;;
  devnet)
    SUI_SCRIPT="dsui"
    ;;
  testnet)
    SUI_SCRIPT="tsui"
    ;;
  active)
    SUI_SCRIPT="asui"
    ;;
  cargobin)
    SUI_SCRIPT="csui"
    SUI_BIN_DIR="$HOME/.cargo/bin"
    ;;
  *)
    SUI_SCRIPT="sui-exec"
    ;;
esac

# Configuration files (often needed for sui CLI calls)
NETWORK_CONFIG="$CONFIG_DATA_DIR/network.yaml"
CLIENT_CONFIG="$CONFIG_DATA_DIR/client.yaml"

# This is the default repo for localnet/devnet/testnet scripts.
# Normally $SUI_REPO_DIR will symlink to $SUI_REPO_DIR_DEFAULT
SUI_REPO_DIR_DEFAULT="$WORKDIRS/$WORKDIR/sui-repo-default"

# This is the default config for localnet/devnet/testnet scripts.
# Normally $CONFIG_DATA_DIR will symlink to CONFIG_DATA_DIR_DEFAULT
CONFIG_DATA_DIR_DEFAULT="$WORKDIRS/$WORKDIR/config-default"

# Location for genesis data for "default" repo.
DEFAULT_GENESIS_DATA_DIR="$SCRIPTS_DIR/genesis_data"

# Location for generated genesis data (on first start after set-sui-repo)
GENERATED_GENESIS_DATA_DIR="$WORKDIRS/$WORKDIR/genesis-data"

# The two shims find in each $WORKDIR
SUI_EXEC="$WORKDIRS/$WORKDIR/sui-exec"
WORKDIR_EXEC="$WORKDIRS/$WORKDIR/workdir-exec"

# Where all the sui.log of the sui client go to die.
SUI_CLIENT_LOG_DIR="$WORKDIRS/$WORKDIR/logs/sui.log"

# Control if network execution, interaction and
# publication are to be mock.
#
# Intended for limited CI tests (github action).
SUI_BASE_NET_MOCK=false
SUI_BASE_NET_MOCK_VER="sui 0.99.99-abcdef"
SUI_BASE_NET_MOCK_PID="999999"

# Add color
function __echo_color() {
	if [[ "${CFG_terminal_color:?}" == 'false' ]]; then
		echo -e -n "$2"
	else
		echo -e -n "\033[$1;$2$3\033[0m"
	fi
}

# Bold colors, mostly used for status.
function echo_black() {
	__echo_color "1" "30m" "$1"
}
export -f echo_black

function echo_red() {
	__echo_color "1" "31m" "$1"
}
export -f echo_red

function echo_green() {
	__echo_color "1" "32m" "$1"
}
export -f echo_green

function echo_yellow() {
	__echo_color "1" "33m" "$1"
}
export -f echo_yellow

function echo_blue() {
	__echo_color "1" "34m" "$1"
}
export -f echo_blue

function echo_magenta() {
	__echo_color "1" "35m" "$1"
}
export -f echo_magenta

function echo_cyan() {
	__echo_color "1" "36m" "$1"
}
export -f echo_cyan

function echo_white() {
	__echo_color "1" "37m" "$1"
}
export -f echo_white

# Low colors, used mostly in --help.
function echo_low_green() {
  __echo_color "0" "32m" "$1"
}

function echo_low_yellow() {
  __echo_color "0" "33m" "$1"
}

# Utility functions.
info_exit() { echo "$*" 1>&2; exit 1; }
export -f info_exit

setup_error() { { echo_red "Error: "; echo "$*"; } 1>&2; exit 1; }
export -f setup_error

warn_user() { { echo_yellow "Warning: "; echo "$*"; } 1>&2; }
export -f warn_user

version_greater_equal()
{
  local _arg1 _arg2
  # Remove everything until first digit
  # Remove trailing "-build number" if specified.
  # Keep only major/minor, ignore minor if specified.
  # shellcheck disable=SC2001
  _arg1=$(echo "$1" | sed 's/^[^0-9]*//; s/-.*//; s/\(.*\)\.\(.*\)\..*/\1.\2/')
  # shellcheck disable=SC2001
  _arg2=$(echo "$2" | sed 's/^[^0-9]*//; s/-.*//; s/\(.*\)\.\(.*\)\..*/\1.\2/')
  printf '%s\n%s\n' "$_arg2" "$_arg1" | sort --check=quiet --version-sort;
}
export -f version_greater_equal

version_less_than()
{
  if version_greater_equal "$1" "$2"; then
    false; return
  fi
  true; return
}
export -f version_less_than

script_cmd() { script -efqa "$SCRIPT_OUTPUT" -c "$*"; }
export -f script_cmd
beginswith() { case $2 in "$1"*) true;; *) false;; esac; }
export -f beginswith

file_newer_than() {
  # Check if file $1 is newer than file $2
  # Return true on any error (assuming need attention).
  if [ ! -f "$1" ] || [ ! -f "$2" ]; then
    true; return
  fi
  local _date1 _date2
  _date1=$(date -r "$1" +%s)
  _date2=$(date -r "$2" +%s)

  if [[ "$_date1" > "$_date2" ]]; then
    true; return
  fi

  false; return
}
export -f file_newer_than

set_key_value() {
  local _KEY=$1
  local _VALUE=$2
  # A key-value persisted in the workdir.
  # The value can't be the string "NULL"
  #
  # About unusual '>|' :
  #   https://stackoverflow.com/questions/4676459/write-to-file-but-overwrite-it-if-it-exists
  if [ -z "$_VALUE" ]; then
    setup_error "Can't write an empty value for [$_KEY]"
  fi
  if [ -z "$_KEY" ]; then
    setup_error "Can't use an empty key for value [$_VALUE]"
  fi
  mkdir -p "$WORKDIRS/$WORKDIR/.state"
  echo "$_VALUE" >| "$WORKDIRS/$WORKDIR/.state/$_KEY"
}
export -f set_key_value

get_key_value() {
  local _KEY=$1
  # A key-value persisted in the workdir.
  # Return the string NULL on error or missing.
  if [ -z "$_KEY" ]; then
    setup_error "Can't retreive empty key"
  fi
  if [ ! -f "$WORKDIRS/$WORKDIR/.state/$_KEY" ]; then
    echo "NULL"; return
  fi

  local _VALUE
  _VALUE=$(cat "$WORKDIRS/$WORKDIR/.state/$_KEY")

  if [ -z "$_VALUE" ]; then
      echo "NULL"; return
  fi

  # Error
  echo "$_VALUE"
}
export -f get_key_value

# Now load all the $CFG_ variables from the sui-base.yaml files.
# shellcheck source=SCRIPTDIR/__parse-yaml.sh
source "$SCRIPTS_DIR/common/__parse-yaml.sh"
update_sui_base_yaml() {
  # Load defaults twice.
  #
  # First with CFG_ prefix, the second with CFGDEFAULT_
  #
  # This allow to detect if there was an override or not (e.g. to re-assure
  # the user in a message that an override was applied).
  #
  YAML_FILE="$SCRIPTS_DIR/defaults/$WORKDIR/sui-base.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval "$(parse_yaml "$YAML_FILE" "CFG_")"
    eval "$(parse_yaml "$YAML_FILE" "CFGDEFAULT_")"
  fi

  # Load overrides from workdir with CFG_ prefix.
  YAML_FILE="$WORKDIRS/$WORKDIR/sui-base.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval "$(parse_yaml "$YAML_FILE" "CFG_")"
  fi
}
export -f update_sui_base_yaml

update_sui_base_yaml;

update_sui_base_version() {
  # Best effort to add the build number to the version.
  # If no success, just use the hard coded major.minor.patch info.
  local _BUILD
  _BUILD=$(if cd "$SCRIPTS_DIR"; then git rev-parse --short HEAD; else echo "-"; fi)
  if [ -n "$_BUILD" ] && [ "$_BUILD" != "-" ]; then
    SUI_BASE_VERSION="$SUI_BASE_VERSION-$_BUILD"
  fi
}
export -f update_sui_base_version

update_sui_base_version;

cd_sui_log_dir() {
  if [ -d "$WORKDIRS/$WORKDIR" ]; then
    mkdir -p "$SUI_CLIENT_LOG_DIR"
    cd "$SUI_CLIENT_LOG_DIR" || setup_error "could not access [ $SUI_CLIENT_LOG_DIR ]"
  fi
}
export -f cd_sui_log_dir

cd_sui_log_dir;

# SUI_BASE_MOCK is a global boolean always defined.
update_SUI_BASE_NET_MOCK_var() {
  # OS_RUNNER is defined when running in github actions.
 if [ -n "$OS_RUNNER" ]; then
   SUI_BASE_NET_MOCK=true
   if [ "$CFG_SUI_BASE_NET_MOCK" = "true" ]; then
     # This is really really bad and would be an accidental commit of a dev
     # test setup. Fix ASAP. It means github found the default configuration
     # is set to mock the network!!!
     #
     # The following exit non-zero (should break the CI)
     setup_error "Bad commit to github. Contact https://sui-base.io devs ASAP to fix the release."
   fi
 fi

 # Mocking can be control with sui-base.yaml.
 #
 # This is undocummented, for sui-base devs only.
 if [ "$CFG_SUI_BASE_NET_MOCK" = "true" ]; then
   SUI_BASE_NET_MOCK=true
 fi

 # If mocking AND state is started, then simulate
 # that the process are already running.
 if $SUI_BASE_NET_MOCK; then
    local _USER_REQUEST
    _USER_REQUEST=$(get_key_value "user_request");
    if [ "$_USER_REQUEST" = "start" ]; then
      export SUI_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
      export SUI_FAUCET_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
    fi
  fi
}
export -f update_SUI_BASE_NET_MOCK_var

update_SUI_BASE_NET_MOCK_var;

check_yaml_parsed()
{
  local _var_name="CFG_$1"
  # Will fail if either not set or empty string. Both
  # wrong in all cases when calling this function.
  if [ -z "${!_var_name}" ]; then
    setup_error "Missing [ $_var_name ] in sui-base.yaml."
  fi

  _var_name="CFGDEFAULT_$1"
  if [ -z "${!_var_name}" ]; then
    setup_error "Missing [ $_var_name ] in *default* sui-base.yaml"
  fi

}
export -f check_yaml_parsed

build_sui_repo_branch() {
  ALLOW_DOWNLOAD="$1";

  # Verify Sui pre-requisites are installed.
  which curl &> /dev/null || setup_error "Need to install curl. See https://docs.sui.io/build/install#prerequisites";
  which git &> /dev/null || setup_error "Need to install git. See https://docs.sui.io/build/install#prerequisites";
  which cmake &> /dev/null || setup_error "Need to install cmake. See https://docs.sui.io/build/install#prerequisites";
  which rustc &> /dev/null || setup_error "Need to install rust. See https://docs.sui.io/build/install#prerequisites";
  which cargo &> /dev/null || setup_error "Need to install cargo. See https://docs.sui.io/build/install#prerequisites";

  # Verify Rust is recent enough.
  version_greater_equal "$(rustc --version)" "$MIN_RUST_VERSION" || setup_error "Upgrade rust to a more recent version";

  if [ "$ALLOW_DOWNLOAD" = "false" ]; then
    if is_sui_repo_dir_override; then
      echo "Skipping git clone/fetch/pull because set-sui-repo is set."
      echo "Building $WORKDIR at [$RESOLVED_SUI_REPO_DIR]"
      if [ ! -d "$RESOLVED_SUI_REPO_DIR" ]; then
        echo "Error: repo not found at [$RESOLVED_SUI_REPO_DIR]"
        echo "Either create this repo, or revert localnet to work with"
        echo "the default repo by typing \"localnet set-sui-repo\"".
        exit
      fi
    fi
  else

    if [ "${CFG_default_repo_url:?}" != "${CFGDEFAULT_default_repo_url:?}" ] ||
       [ "${CFG_default_repo_branch:?}" != "${CFGDEFAULT_default_repo_branch:?}" ]; then
      echo "sui-base.yaml: Using repo [ $CFG_default_repo_url ] branch [ $CFG_default_repo_branch ]"
    fi

    # If not already done, initialize the default repo.
    if [ ! -d "$SUI_REPO_DIR_DEFAULT" ]; then
      git clone -b "$CFG_default_repo_branch" "$CFG_default_repo_url" "$SUI_REPO_DIR_DEFAULT"  || setup_error "Failed cloning branch [$CFG_default_repo_branch] from [$CFG_default_repo_url]";
      set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT";
    fi

    # Add back the default sui-repo link in case it was deleted.
    if [ ! -L "$SUI_REPO_DIR" ]; then
      set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT";
    fi

    # Force git reset  if this is the very first time cloning (cover for
    # some scenario where the user Ctrl-C in middle of initial git object
    # fetching).
    local _FORCE_GIT_RESET=false
    if [ ! -d "$SUI_REPO_DIR/target" ]; then
      _FORCE_GIT_RESET=true
    fi

    # Update sui devnet local repo (if needed)
    #(cd "$SUI_REPO_DIR" && git switch "$CFG_default_repo_branch" >& /dev/null)

    local _BRANCH
    _BRANCH=$(cd "$SUI_REPO_DIR" && git branch --show-current)
    if [ "$_BRANCH" != "$CFG_default_repo_branch" ]; then
      (cd "$SUI_REPO_DIR" && git checkout -f "$CFG_default_repo_branch" >& /dev/null)
    fi
    (cd "$SUI_REPO_DIR" && git remote update >& /dev/null)
    V1=$(if cd "$SUI_REPO_DIR"; then git rev-parse HEAD; else setup_error "unexpected missing $SUI_REPO_DIR"; fi)
    V2=$(if cd "$SUI_REPO_DIR"; then git rev-parse '@{u}'; else setup_error "unexpected missing $SUI_REPO_DIR"; fi)
    if [ "$V1" != "$V2" ]; then
      _FORCE_GIT_RESET=true
    fi

    if $_FORCE_GIT_RESET; then
      # Does a bit more than needed, but should allow to recover
      # from most operator error...
      echo Updating sui "$WORKDIR" in sui-base...
      (cd "$SUI_REPO_DIR" && git fetch > /dev/null)
      (cd "$SUI_REPO_DIR" && git reset --hard origin/"$CFG_default_repo_branch" > /dev/null)
      (cd "$SUI_REPO_DIR" && git merge '@{u}')
    fi
    echo "Building $WORKDIR from latest repo [$CFG_default_repo_url] branch [$CFG_default_repo_branch]"
  fi

  (if cd "$SUI_REPO_DIR"; then cargo build; else setup_error "unexpected missing $SUI_REPO_DIR"; fi)

  # Sanity test that the sui binary works
  if [ ! -f "$SUI_BIN_DIR/sui" ]; then
    setup_error "$SUI_BIN_DIR/sui binary not found"
  fi

  update_SUI_VERSION_var;

  # Check if sui is recent enough.
  version_greater_equal "$SUI_VERSION" "$MIN_SUI_VERSION" || setup_error "Sui binary version too old (not supported)"
}

export -f build_sui_repo_branch

exit_if_not_installed() {
  # Help the user that did not even do the installation of the symlinks
  # and is trying instead to call directly from "~/sui-base/scripts"
  # (which will cause some trouble with some script).
  case "$SCRIPT_NAME" in
  "asui"|"lsui"|"csui"|"dsui"|"tsui"|"localnet"|"devnet"|"testnet"|"workdirs")
    if [ ! -L "$LOCAL_BIN/$SCRIPT_NAME" ]; then
      echo
      echo "Some sui-base files are missing. The installation was"
      echo "either not done or failed."
      echo
      echo "Run ~/sui-base/install again to fix this."
      echo
      exit 1
    fi
    ;;
  *) ;;
  esac

  # TODO Test sui-base on $PATH is fine.
}
export -f exit_if_not_installed

exit_if_workdir_not_ok() {
  # This is a common "operator" error (not doing command in right order).
  if ! is_workdir_ok; then
    if [ "$WORKDIR" = "cargobin" ]; then
      exit_if_sui_binary_not_ok; # Point to a higher problem (as needed).
      echo "cargobin workdir not initialized"
      echo
      echo "Please run ~/sui-base/.install again to detect"
      echo "the ~/.cargo/bin/sui and create the cargobin workdir."
      echo
      echo "It is safe to re-run ~/sui-base/.install when sui-base"
      echo "is already installed (it just installs what is missing)."
    else
      echo "$WORKDIR workdir not initialized"
      echo
      echo "Do one of the following:"
      if [ ! -d "$WORKDIRS/WORKDIR" ]; then
        echo "        $WORKDIR start (recommended)"
        echo "        $WORKDIR create"
      else
        if [ "$CFG_network_type" = "local" ]; then
          echo "        $WORKDIR regen  (recommended)"
          echo "        $WORKDIR update"
        else
          echo "        $WORKDIR update  (recommended)"
        fi
        echo "        $WORKDIR start"
      fi
      echo
      echo "Type \"$WORKDIR --help\" for more options"
    fi
    exit 1
  fi
}
export -f exit_if_workdir_not_ok

exit_if_sui_binary_not_ok() {
  # This is for common "operator" error (not doing command in right order).
  local _BIN_NOT_FOUND=false
  if [ ! -f "$SUI_BIN_DIR/sui" ]; then
    _BIN_NOT_FOUND=true
  else
    update_SUI_VERSION_var; # Note: Requires $SUI_BIN_DIR/sui
    if version_greater_equal "$SUI_VERSION" "0.27"; then
      if [ ! -f "$SUI_BIN_DIR/sui-faucet" ]; then
        _BIN_NOT_FOUND=true
      fi
    fi
  fi

  if [ "$_BIN_NOT_FOUND" = "true" ]; then
    if [ "$WORKDIR" = "cargobin" ]; then
      echo "The $HOME/.cargo/bin/sui was not found."
      echo "Follow Mysten Lab procedure to install it:"
      echo " https://docs.sui.io/build/install#install-sui-binaries"
    else
      echo
      echo "The sui binary for $WORKDIR was not found."
      echo
      echo " Do one of the following to build it:"
      echo "    $WORKDIR start"
      echo "    $WORKDIR update"
      echo
    fi
    exit 1
  fi

  # Sometimes the binary are ok, but not the config (may happen when the
  # localnet config directory is safely wipe out on set-sui-repo transitions).
  if [ "$CFG_network_type" = "local" ]; then
    if  [ ! -f "$NETWORK_CONFIG" ] || [ ! -f "$CLIENT_CONFIG" ]; then
      echo
      echo "The localnet need to be regenerated."
      echo
      echo " Do one of the following:"
      echo "    $WORKDIR regen"
      echo "    $WORKDIR update"
      echo
      exit 1
    fi
  fi
}
export -f exit_if_sui_binary_not_ok

is_sui_binary_ok() {
  # Keep this one match the logic of exit_if_sui_binary_not_ok()
  # The difference is this function should NEVER exit because it
  # is used to detect problems and have the caller try to repair the
  # binary.
  if [ ! -f "$SUI_BIN_DIR/sui" ]; then
    false; return
  fi

  # Get the version, but in a way that would not exit on failure.
  cd_sui_log_dir;
  local _SUI_VERSION_ATTEMPT
  _SUI_VERSION_ATTEMPT=$("$SUI_BIN_DIR/sui" --version)
  # TODO test here what would really happen on corrupted binary...
  if [ -z "$_SUI_VERSION_ATTEMPT" ]; then
    false; return
  fi

  if version_greater_equal "$_SUI_VERSION_ATTEMPT" "0.27"; then
    if [ ! -f "$SUI_BIN_DIR/sui-faucet" ]; then
      false; return
    fi
  fi

  if [ "$CFG_network_type" = "local" ]; then
    if  [ ! -f "$NETWORK_CONFIG" ] || [ ! -f "$CLIENT_CONFIG" ]; then
      false; return
    fi
  fi

  true; return
}
export -f is_sui_binary_ok

check_workdir_ok() {
  # Sanity check the workdir looks operational.
  #
  # This should be done early for most script call.
  #
  # This is to minimize support/confusion. First, get the setup right
  # before letting the user do more damage...
  if [ "$WORKDIR" = "cargobin" ]; then
    # Special case because no repo etc... (later to be handled better by sui-base.yaml)
    # Just check for the basic.
    if [ ! -f "$HOME/.cargo/bin/sui" ]; then
      setup_error "This script is for user who choose to install ~/.cargo/bin/sui. You do not have it installed."
    fi

    if [ ! -d "$WORKDIRS" ]; then
      setup_error "$WORKDIRS missing. Please run '~/sui-base/install' to repair"
    fi

    if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
      setup_error "$WORKDIRS/$WORKDIR missing. Please run '~/sui-base/install' to repair"
    fi
    # Success
    return
  fi

  if [ ! -d "$WORKDIRS" ]; then
    setup_error "$WORKDIRS missing. Please run '$WORKDIR update' first"
  fi

  if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
    setup_error "$WORKDIRS/$WORKDIR missing. Please run '$WORKDIR update' first"
  fi

  if [ ! -d "$SUI_REPO_DIR" ]; then
    setup_error "$SUI_REPO_DIR missing. Please run '$WORKDIR update' first"
  fi

  if [ "$CFG_network_type" = "remote" ]; then
    # Good enough for workdir like devnet/testnet.
    return
  fi

  if [ ! -f "$NETWORK_CONFIG" ]; then
    setup_error "$NETWORK_CONFIG missing. Please run '$WORKDIR update' first"
  fi

  if [ ! -f "$CLIENT_CONFIG" ]; then
    setup_error "$CLIENT_CONFIG missing. Please run '$WORKDIR update' first"
  fi
}
export -f check_workdir_ok

is_workdir_ok() {
  # Just check if enough present on the filesystem to allow configuration, not
  # if there is enough for running the sui client.
  #
  # In other word, detect if at least the "create" command was performed.
  if [ ! -d "$WORKDIRS" ]; then
    false; return;
  fi

  if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
    false; return;
  fi

  if [ ! -f "$WORKDIRS/$WORKDIR/sui-base.yaml" ] ||
     [ ! -f "$WORKDIRS/$WORKDIR/sui-exec" ] ||
     [ ! -f "$WORKDIRS/$WORKDIR/workdir-exec" ]; then
    false; return;
  fi

  if [ ! -L "$WORKDIRS/$WORKDIR/config" ]; then
    false; return;
  fi

  true; return
}
export -f is_workdir_ok

update_exec_shim() {
  WORKDIR_PARAM="$1"
  FILENAME="$2"

  # Create the sui-exec file (if does not exists)
  if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/$FILENAME" ]; then
    cp "$SCRIPTS_DIR/templates/$FILENAME" "$WORKDIRS/$WORKDIR_PARAM/$FILENAME"
  else
    # Exists, check if need to be "upgraded".
    # Do a byte cmp... templates are intended to be small shims or yaml, should
    # not take long and this is only part of relatively "heavy" operation, such
    # as update/regen of a workdir.
    # If worried, then replace with md5sum one day?
    cmp --silent "$SCRIPTS_DIR/templates/$FILENAME" "$WORKDIRS/$WORKDIR_PARAM/$FILENAME" || {
      cp -f "$SCRIPTS_DIR/templates/$FILENAME" "$WORKDIRS/$WORKDIR_PARAM/$FILENAME"
    }
  fi
}
export -f update_exec_shim

create_exec_shims_as_needed() {
  WORKDIR_PARAM="$1"
  update_exec_shim "$WORKDIR_PARAM" "sui-exec"
  update_exec_shim "$WORKDIR_PARAM" "workdir-exec"
}
export -f create_exec_shims_as_needed

create_active_symlink_as_needed() {
  WORKDIR_PARAM="$1"

  # Protect against self-reference.
  if [ "$WORKDIR_PARAM" == "active" ]; then
    return
  fi

  # Create a new active symlink, but not overwrite an
  # existing one (because it represents the user intent).
  if [ ! -L "$WORKDIRS/active" ]; then
     set_active_symlink_force "$WORKDIR_PARAM"
  fi
}
export -f create_active_symlink_as_needed

create_config_symlink_as_needed() {
  WORKDIR_PARAM="$1"
  TARGET_CONFIG="$2"

  # Create a new config symlink, but not overwrite an
  # existing one (because it represents the user intent).
  if [ ! -L "$WORKDIRS/$WORKDIR_PARAM/config" ]; then
     set_config_symlink_force "$WORKDIR_PARAM" "$TARGET_CONFIG"
  fi
}
export -f create_config_symlink_as_needed

create_state_dns_as_needed() {
  WORKDIR_PARAM="$1"

  if [ "$WORKDIR_PARAM" = "cargobin" ]; then
    return
  fi

  # Create/repair (if possible)
  if [ ! -d "$WORKDIRS/$WORKDIR_PARAM/.state" ]; then
    mkdir -p "$WORKDIRS/$WORKDIR_PARAM/.state"
  fi

  if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/.state/dns" ]; then
    # Just transform the human friendly recovery.txt into
    # a JSON file and generate names along the way.
    local _SRC="$WORKDIRS/$WORKDIR_PARAM/config/recovery.txt"
    if [ -f "$_SRC" ]; then
      {
        echo "{ \"known\": {"
        local _KEY_SCHEME_LIST=("ed25519" "secp256k1" "secp256r1")
        local _FIRST_LINE=true
        for scheme in "${_KEY_SCHEME_LIST[@]}"; do
          local _LINES
          _LINES=$(grep -i "$scheme" "$_SRC" | sort | sed 's/.*\[\(.*\)\]/\1/')
          (( i=1 ))
          while IFS= read -r line; do
            if ! $_FIRST_LINE; then echo ","; else _FIRST_LINE=false; fi
            echo -n "\"sb-$i-$scheme\": { \"address\": \"$line\" }"
            (( i++ ))
          done < <(printf '%s\n' "$_LINES")
        done
        echo
        echo "}}"
      } >> "$WORKDIRS/$WORKDIR_PARAM/.state/dns"
    fi
  fi
}
export -f create_state_dns_as_needed

create_state_links_as_needed() {
  WORKDIR_PARAM="$1"

  if [ "$WORKDIR_PARAM" = "cargobin" ]; then
    return
  fi

  # Create/repair (if possible)
  if [ ! -d "$WORKDIRS/$WORKDIR_PARAM/.state" ]; then
    mkdir -p "$WORKDIRS/$WORKDIR_PARAM/.state"
  fi

  # Take the user sui-base.yaml and create the .state/links
  #
  # Eventually the state will be updated through health
  # monitoring of the links, not just this "mindless"
  # initialization.
  #
  # Note: .state/links changes will be implemented with "atomic filesystem mv",
  #       not as a file/inode modification.
  #

  # Find how many links, and create the primary and secondary id
  # references.
  if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/.state/links" ] || file_newer_than "$WORKDIRS/$WORKDIR_PARAM/sui-base.yaml" "$WORKDIRS/$WORKDIR_PARAM/.state/links"; then
    # parse_yaml ~/sui-base/scripts/defaults/testnet/sui-base.yaml;
    check_yaml_parsed links_;

    (( _n_links=0 ))
    for _links in ${CFG_links_:?} ; do
      (( _n_links++))
    done

    if [ $_n_links -eq 0 ]; then
      setup_error "No links found in $WORKDIRS/$WORKDIR_PARAM/config/sui-base.yaml"
    fi

    rm -rf "$WORKDIRS/$WORKDIR_PARAM/.state/links.tmp"
    {
      echo "{"
      echo -n "\"selection\": { \"primary\": 0, "

      echo -n "\"secondary\": "
      if [ $_n_links -ge 2 ]; then
        echo -n "1"
      else
        echo -n "0"
      fi
      echo -n ", "

      echo "\"n_links\": $_n_links }, "

      echo "\"links\": ["
      _FIRST_LINE=true
      (( _i=0 ))
      for _links in ${CFG_links_:?} ; do
        if ! $_FIRST_LINE; then echo ","; else _FIRST_LINE=false; fi
        echo "    {"
        echo "      \"id\": $_i, "
        # shellcheck disable=SC2001
        _alias=$(echo "${!_links}" | sed 's/.*alias.*:\(.*\)/\1/' | tr -d '[:space:]')
        echo "      \"alias\": \"$_alias\", "

        _rpc="${_links}_rpc"
        echo "      \"rpc\": \"${!_rpc}\", "

        _ws="${_links}_ws"
        echo "      \"ws\": \"${!_ws}\""
        (( _i++ ))
        echo -n "    }"; # close link
      done
      echo
      echo "  ]"; # close links
      echo "}";
    } > "$WORKDIRS/$WORKDIR_PARAM/.state/links.tmp"
    # Atomic mv assuming Linux ext4 (good enough for now)
    mv -f "$WORKDIRS/$WORKDIR_PARAM/.state/links.tmp" "$WORKDIRS/$WORKDIR_PARAM/.state/links"
  fi
}
export -f create_state_links_as_needed

create_state_as_needed() {
  WORKDIR_PARAM="$1"

  # Create/repair
  if [ ! -d "$WORKDIRS/$WORKDIR_PARAM/.state" ]; then
    mkdir -p "$WORKDIRS/$WORKDIR_PARAM/.state"
  fi

  if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/.state/user_request" ]; then
    set_key_value "user_request" "stop"
  fi

  if [ "$WORKDIR_PARAM" != "active" ]; then
    if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/.state/name" ]; then
      set_key_value "name" "$WORKDIR_PARAM"
    fi
  fi

  create_state_dns_as_needed "$WORKDIR_PARAM";
  create_state_links_as_needed "$WORKDIR_PARAM";
}
export -f create_state_as_needed

repair_workdir_as_needed() {
  WORKDIR_PARAM="$1"

  mkdir -p "$WORKDIRS"

  if [ "$WORKDIR_PARAM" = "active" ]; then
    update_ACTIVE_WORKDIR_var;
    if [ -z "$ACTIVE_WORKDIR" ] || [ ! -d "$WORKDIRS/$ACTIVE_WORKDIR" ]; then
      # Do not create an "active" directory, but...
      return
    fi
    # ... keep going to repair if pointing to a valid directory.
    WORKDIR_PARAM="$ACTIVE_WORKDIR"
  else
    if [ ! -d "$WORKDIRS/$WORKDIR_PARAM" ]; then
      # "Create" using the template.
      cp -r "$SCRIPTS_DIR/templates/$WORKDIR_PARAM" "$WORKDIRS"
    fi
    create_active_symlink_as_needed "$WORKDIR_PARAM";
  fi

  create_exec_shims_as_needed "$WORKDIR_PARAM";
  create_state_as_needed "$WORKDIR_PARAM";

  if [ "$WORKDIR_PARAM" = "cargobin" ]; then
    create_config_symlink_as_needed "$WORKDIR_PARAM" "$HOME/.sui/sui_config"
  else
    create_config_symlink_as_needed "$WORKDIR_PARAM" "$WORKDIRS/$WORKDIR_PARAM/config-default"
  fi

  cd_sui_log_dir; # Create the sui.log directory as needed and make it the current one.
}
export -f repair_workdir_as_needed

set_active_symlink_force() {
  WORKDIR_PARAM="$1"
  # Create or force the active symlink to the specified target.
  if [ ! -L "$WORKDIRS/active" ]; then
    ln -s "$WORKDIRS/$WORKDIR_PARAM" "$WORKDIRS/active"
  else
    update_ACTIVE_WORKDIR_var;
    if [ "$ACTIVE_WORKDIR" != "$WORKDIR_PARAM" ]; then
      ln -nsf "$WORKDIRS/$WORKDIR_PARAM" "$WORKDIRS/active"
      update_ACTIVE_WORKDIR_var;
    fi
  fi
}
export -f set_active_symlink_force

set_config_symlink_force() {
  WORKDIR_PARAM="$1"
  TARGET_DIR="$2"
  # Create or force the active symlink to the specified target.
  if [ ! -L "$WORKDIRS/$WORKDIR_PARAM/config" ]; then
    ln -s "$TARGET_DIR" "$WORKDIRS/$WORKDIR_PARAM/config"
  else
    RESOLVED_DIR="$(readlink -f "$WORKDIRS/$WORKDIR_PARAM/config")"
    if [ "$RESOLVED_DIR" != "$TARGET_DIR" ]; then
      ln -nsf "$TARGET_DIR" "$WORKDIRS/$WORKDIR_PARAM/config"
    fi
  fi
}
export -f set_config_symlink_force

update_ACTIVE_WORKDIR_var() {
  # Identify the active workdir, if any (deduce from the symlink).
  if [ ! -L "$WORKDIRS/active" ]; then
    unset ACTIVE_WORKDIR
  else
    RESOLVED_PATH="$(readlink -f "$WORKDIRS/active")"
    ACTIVE_WORKDIR="$(basename "$RESOLVED_PATH")"
  fi
}
export -f update_ACTIVE_WORKDIR_var

get_process_pid() {
  local _PROC="$1"
  local _ARGS="$2"
  local _PID
  # Given a process "string" return the pid as a string.
  # Return NULL if not found.
  #
  # Details on the cryptic parsing:
  #   Get ps with "sui start" in its command line, grep exclude itself from the list, head takes the first process (should
  #   not be more than one) the 1st sed remove leading space, the 2nd sed split words into line and finally the pid is the
  #   word on the first/head line.
  #
  if [[ $(uname) == "Darwin" ]]; then
    _PID=$(ps x -o pid,comm | grep "$_PROC" | grep -v grep | head -n 1 | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | head -n 1)
  else
    _PID=$(ps x -o pid,cmd | grep "$_PROC $_ARGS" | grep -v grep | head -n 1 | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | head -n 1)
  fi

  if [ -n "$_PID" ]; then
    echo "$_PID"
  else
    echo "NULL"
  fi
}
export -f get_process_pid

update_SUI_PROCESS_PID_var() {
  if $SUI_BASE_NET_MOCK; then return; fi

  # Useful to check if the sui process is running (this is the parent for the "localnet")
  local _PID
  _PID=$(get_process_pid "sui" "start")
  if [ "$_PID" = "NULL" ]; then
    unset SUI_PROCESS_PID
  else
    SUI_PROCESS_PID=$_PID
  fi
}
export -f update_SUI_PROCESS_PID_var

update_SUI_VERSION_var() {
  # Take note that $SUI_BIN_DIR here is used to properly consider if the
  # context of the script is localnet, devnet, testnet, mainnet... (they
  # are not the same binaries and versions).
  cd_sui_log_dir;

  if $SUI_BASE_NET_MOCK; then
    SUI_VERSION=$SUI_BASE_NET_MOCK_VER
    return
  fi

  SUI_VERSION=$("$SUI_BIN_DIR/sui" --version)
  if [ -z "$SUI_VERSION" ]; then
    setup_error "$SUI_BIN_DIR/sui --version not running as expected"
  fi
}
export -f update_SUI_VERSION_var

stop_sui_process() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUI_PROCESS_PID_var;
  if [ -n "$SUI_PROCESS_PID" ]; then
    echo "Stopping $WORKDIR (process pid $SUI_PROCESS_PID)"
    if $SUI_BASE_NET_MOCK; then
      unset SUI_PROCESS_PID
    else
      if [[ $(uname) == "Darwin" ]]; then
        kill -9 "$SUI_PROCESS_PID"
      else
        skill -9 "$SUI_PROCESS_PID"
      fi
    fi

    # Make sure it is dead.
    end=$((SECONDS+15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_SUI_PROCESS_PID_var;
      if [ -z "$SUI_PROCESS_PID" ]; then
        break
      else
        echo -n "."
        sleep 1
        AT_LEAST_ONE_SECOND=true
      fi
    done

    # Just UI aesthetic newline for when there was "." printed.
    if [ "$AT_LEAST_ONE_SECOND" = true ]; then
      echo
    fi

    if [ -n "$SUI_PROCESS_PID" ]; then
      setup_error "Sui process pid=$SUI_PROCESS_PID still running. Try again, or stop (kill) the sui process yourself before proceeding."
    fi
  fi
}
export -f stop_sui_process

start_sui_process() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already started.

  exit_if_sui_binary_not_ok;
  cd_sui_log_dir;
  update_SUI_PROCESS_PID_var;
  if [ -z "$SUI_PROCESS_PID" ]; then
    echo "Starting localnet process"
    if $SUI_BASE_NET_MOCK; then
      SUI_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
    else
      "$SUI_BIN_DIR/sui" start --network.config "$NETWORK_CONFIG" >& "$CONFIG_DATA_DIR/sui-process.log" &
    fi
    #NEW_PID=$!

    # Loop until "sui client" confirms to be working, or exit if that takes
    # more than 30 seconds.
    end=$((SECONDS+60))
    (( _mid_message=30 ))
    ALIVE=false
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      if $SUI_BASE_NET_MOCK; then
        CHECK_ALIVE="some output"
      else
        CHECK_ALIVE=$("$SUI_BIN_DIR/sui" client --client.config "$CLIENT_CONFIG" objects | grep -i Digest)
      fi
      if [ -n "$CHECK_ALIVE" ]; then
        ALIVE=true
        break
      else
        echo -n "."
        sleep 1
        AT_LEAST_ONE_SECOND=true
      fi
      if [ $_mid_message -ge 1 ]; then
        if [ $_mid_message -eq 1 ]; then
          echo -n "(may take some time on slower system)"
        fi
        (( --_mid_message ))
      fi
    done

    # Just UI aesthetic newline for when there was "." printed.
    if [ "$AT_LEAST_ONE_SECOND" = true ]; then
      echo
    fi

    # Act on success/failure of the sui process responding to "sui client".
    if [ "$ALIVE" = false ]; then
      echo "Sui process not responding. Try again? (may be the host is too slow?)."
      exit;
    fi

    update_SUI_PROCESS_PID_var;
    echo "localnet started (process pid $SUI_PROCESS_PID)"
    update_SUI_VERSION_var;
    echo "$SUI_VERSION"
  fi
}
export -f start_sui_process

ensure_client_OK() {
  # This is just in case the user switch the envs on the clients instead of simply using
  # the scripts... we have then to fix things up here. Not an error unless the fix fails.
  cd_sui_log_dir;
  # TODO Add paranoiac validation, fix the URL part, for now this is used only for localnet.
  #if [ "$CFG_network_type" = "local" ]; then
    # Make sure localnet exists in sui envs (ignore errors because likely already exists)
    #echo $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" new-env --alias $WORKDIR --rpc http://0.0.0.0:9000
    "$SUI_BIN_DIR/sui" client --client.config "$CLIENT_CONFIG" new-env --alias "$WORKDIR" --rpc "$CFG_links_1_rpc" >& /dev/null

    # Make localnet the active envs (should already be done, just in case, do it again here).
    #echo $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" switch --env $WORKDIR
    "$SUI_BIN_DIR/sui" client --client.config "$CLIENT_CONFIG" switch --env "$WORKDIR" >& /dev/null
  #fi
}
export -f ensure_client_OK

publish_clear_output() {
  local _DIR=$1
  # Only clear potential last publication.
  rm -f "$_DIR/publish-output.txt"
  rm -f "$_DIR/publish-output.json"
  rm -f "$_DIR/created-objects.json"
  rm -f "$_DIR/package-id.json"
}
export -f publish_clear_output

# Check if there is a Move.toml at specified (parameter) directory.
#
# If not found, then look deeper at move/Move.toml.
#
# If not found, the variable is unset.
#
# When found, while at it, update also the following variable:
#   MOVE_TOML_PACKAGE_NAME
update_MOVE_TOML_DIR_var() {
  unset MOVE_TOML_DIR
  unset MOVE_TOML_PACKAGE_NAME

  if [ -f "$1/Move.toml" ]; then
    MOVE_TOML_DIR=$1
  else
    if [ -f "$1/move/Move.toml" ]; then
      MOVE_TOML_DIR=$1/move
    fi
  fi

  if [ -n "$MOVE_TOML_DIR" ]; then
    # Extract "name" key from toml, we can do something less hackish if ever worth it...
    # ... reality is the whole publish/upgrade process will likely change by mainnet.
    MOVE_TOML_PACKAGE_NAME=$(sed -n '/^name *=* */{s///;s/^"//;s/"$//;p;}' "$MOVE_TOML_DIR/Move.toml")
  fi
}
export -f update_MOVE_TOML_DIR_var


# Verify if $SUI_REPO_DIR symlink is toward a user repo (not default).
#
# false if the symlink does not exist.
#
# (Note: does not care if the target directory exists).
is_sui_repo_dir_override() {
  # Verify if symlink resolves and is NOT toward the default.
  if [ -L "$SUI_REPO_DIR" ]; then
    RESOLVED_SUI_REPO_DIR=$(readlink "$SUI_REPO_DIR")
    if [ "$RESOLVED_SUI_REPO_DIR" != "$SUI_REPO_DIR_DEFAULT" ]; then
      true; return;
    fi
  else
    unset RESOLVED_SUI_REPO_DIR
  fi
  false; return;
}
export -f is_sui_repo_dir_override

is_sui_repo_dir_default() {
  # Just negate is_sui_repo_dir_override
  if is_sui_repo_dir_override; then
    false; return
  fi
  true; return
}
export -f is_sui_repo_dir_default

set_sui_repo_dir_default() {
  if is_sui_repo_dir_override; then
    rm -f "$SUI_REPO_DIR"
    echo "Removed set-sui-repo [$RESOLVED_SUI_REPO_DIR]"
  fi

  # Link to the default directory if already exists.
  if [ -d "$SUI_REPO_DIR_DEFAULT" ]; then
    set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT";
  else
    # No default directory.
    # Still a success as long the symlink is gone.
    echo "$WORKDIR using default sui repo"
  fi
}
export -f set_sui_repo_dir_default

set_sui_repo_dir() {

  OPTIONAL_PATH="$@"

  # User errors?
  if [ ! -d "$OPTIONAL_PATH" ]; then
    setup_error "Path [ $OPTIONAL_PATH ] not found"
  fi

  # The -n is important because target is a directory and without it
  # the command line arguments would be interpreted in the 3rd form
  # described in "man ln".
  ln -nsf "$OPTIONAL_PATH" "$SUI_REPO_DIR"

  # Verify success.
  if is_sui_repo_dir_default; then
    echo "$WORKDIR using default sui repo [ $OPTIONAL_PATH ]"
  else
    if is_sui_repo_dir_override; then
      echo "$WORKDIR set-sui-repo is now [ $OPTIONAL_PATH ]"
    else
      setup_error "$WORKDIR set-sui-repo failed [ $OPTIONAL_PATH ]";
    fi
  fi
}
export -f set_sui_repo_dir

create_cargobin_as_needed() {
  # Will check to install/repair cargobin workdir.
  # NOOP when already installed.
  #
  # Reminder: Do not assume $WORKDIR is cargobin at any point
  #           while this function is run (the workdir context
  #           may not exist yet or be something else).
  if [ ! -f "$HOME/.cargo/bin/sui" ]; then
    return
  fi

  if [ ! -d "$WORKDIRS/cargobin" ]; then
    workdir_was_missing=true
  else
    workdir_was_missing=false
  fi

  # Always called, because can do both create/repair.
  repair_workdir_as_needed "cargobin"

  if [ "$workdir_was_missing" = true ]; then
    if [ -d "$WORKDIRS/cargobin" ]; then
      echo "Created workdir for existing ~/.cargo/bin/sui client"
    else
      echo "Warning: workdir creation for ~/.cargo/bin/sui client failed."
    fi
  fi
}
export -f create_cargobin_as_needed

exit_if_not_valid_sui_address() {
  local _ADDR="$1"
  local _SUI_ERR
  # Use the client itself to verify the string is a valid sui address.
  # Inefficient... but 100% sure the check will be compatible with *this* binary.
  _SUI_ERR=$("$SUI_EXEC" client gas "$1" --json 2>&1 | grep -iE "error|invalid|help" )
  if [ -n "$_SUI_ERR" ]; then
    echo "Invalid hexadecimal Sui Address [$1]."
    exit 1
  fi
}
export -f exit_if_not_valid_sui_address

add_test_addresses() {
  # Add 15 addresses to the specified client.yaml (sui.keystore)
  # The _WORDS_FILE is what is normally appended in "recovery.txt"
  local _SUI_BINARY=$1
  local _CLIENT_FILE=$2
  local _WORDS_FILE=$3

  {
    for _ in {1..5}; do
      $_SUI_BINARY client --client.config "$_CLIENT_FILE" new-address ed25519;
      echo ============================; echo; echo;
      $_SUI_BINARY client --client.config "$_CLIENT_FILE" new-address secp256k1;
      echo ============================; echo; echo;
      $_SUI_BINARY client --client.config "$_CLIENT_FILE" new-address secp256r1;
      echo ============================; echo; echo;
    done
  } >> "$_WORDS_FILE";

  # Set highest address as active. Best-effort... no major issue if fail (I assume).
  local _HIGH_ADDR
  _HIGH_ADDR=$($_SUI_BINARY client --client.config "$_CLIENT_FILE" addresses | grep "0x" | sort -r | head -n 1 | awk '{print $1}')
   $_SUI_BINARY client --client.config "$_CLIENT_FILE" switch --address "$_HIGH_ADDR" >& /dev/null
}
export -f add_test_addresses