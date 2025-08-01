# shellcheck shell=bash

# Do not call this script directly. It is a "common script" sourced by other suibase scripts.
#
# It initializes a bunch of environment variable, verify that some initialization took
# place, identify some common user errors etc...

# shellcheck disable=SC2155
export USER_CWD=$(pwd -P)

# Format is: "major.minor.patch-build"
#
# The build hash is best-effort appended later.
export SUIBASE_VERSION="0.1.9"

# Suibase does not work with version below these.
export MIN_SUI_VERSION="sui 0.27.0"
export MIN_RUST_VERSION="rustc 1.81.0"

# Mandatory command line:
#    $1 : Should be the "$0" of the caller script.
#    $2 : Should be the workdir string (e.g. "active", "localnet"... )
# shellcheck disable=SC2155
export SCRIPT_PATH="$(dirname "$1")"
export SCRIPT_NAME
# shellcheck disable=SC2155
export SCRIPT_NAME="$(basename "$1")"
export WORKDIR="$2"

# Detect if the script is called from an unexpected symlink.
if [[ "$SCRIPT_PATH" = *"sui-base"* ]]; then
  if [ -f "$HOME/suibase/repair" ]; then
    ("$HOME/suibase/repair")
  else
    if [ -f "$HOME/sui-base/repair" ]; then
      ("$HOME/sui-base/repair")
    else
      echo "Cannot find repair script. Contact developer for help."
    fi
  fi
  exit 1
fi

# Two key directories location.
export SUIBASE_DIR="$HOME/suibase"
export WORKDIRS="$SUIBASE_DIR/workdirs"

# Some other commonly used locations.
export LOCAL_BIN_DIR="$HOME/.local/bin"
export SCRIPTS_DIR="$SUIBASE_DIR/scripts"
export SUI_REPO_DIR="$WORKDIRS/$WORKDIR/sui-repo"
export CONFIG_DATA_DIR="$WORKDIRS/$WORKDIR/config"
export PUBLISHED_DATA_DIR="$WORKDIRS/$WORKDIR/published-data"
export FAUCET_DIR="$WORKDIRS/$WORKDIR/faucet"
export SUI_BIN_DIR="$SUI_REPO_DIR/target/debug"
export WALRUS_BIN_DIR="$WORKDIRS/$WORKDIR/bin"
export SITE_BUILDER_BIN_DIR="$WORKDIRS/$WORKDIR/bin"

# Suibase binaries are common to all workdirs, therefore are
# installed in a common location.
export SUIBASE_BIN_DIR="$WORKDIRS/common/bin"
export SUIBASE_LOGS_DIR="$WORKDIRS/common/logs"
export SUIBASE_TMP_DIR="/tmp/.suibase"

export SUIBASE_DAEMON_NAME="suibase-daemon"
export SUIBASE_DAEMON_BUILD_DIR="$SUIBASE_DIR/rust/suibase"
export SUIBASE_DAEMON_BIN="$SUIBASE_BIN_DIR/$SUIBASE_DAEMON_NAME"

export DTP_DAEMON_NAME="dtp-daemon"
export DTP_DAEMON_BUILD_DIR="$SUIBASE_DIR/rust/suibase"
export DTP_DAEMON_BIN="$SUIBASE_BIN_DIR/$DTP_DAEMON_NAME"

export DEBUG_PARAM=false # Become true when --debug on command line.

# Prefix often used when calling sui client.
SUI_BIN_ENV="env SUI_CLI_LOG_FILE_ENABLE=1"

export WORKDIR_NAME="$WORKDIR"
export SUI_SCRIPT
export WALRUS_SCRIPT
export SITE_BUILDER_SCRIPT
case $WORKDIR in
localnet)
  SUI_SCRIPT="lsui"
  ;;
devnet)
  SUI_SCRIPT="dsui"
  ;;
testnet)
  SUI_SCRIPT="tsui"
  WALRUS_SCRIPT="twalrus"
  SITE_BUILDER_SCRIPT="tsite"
  ;;
mainnet)
  SUI_SCRIPT="msui"
  WALRUS_SCRIPT="mwalrus"
  SITE_BUILDER_SCRIPT="msite"
  ;;
active)
  SUI_SCRIPT="asui"
  if [ -f "$WORKDIRS/$WORKDIR/.state/name" ]; then
    # Resolve the 'active' workdir link into its target name.
    # Empty string on error.
    WORKDIR_NAME=$(cat "$WORKDIRS/$WORKDIR/.state/name" 2>/dev/null)
  fi
  ;;
cargobin)
  SUI_SCRIPT="csui"
  SUI_BIN_DIR="$HOME/.cargo/bin"
  ;;
*)
  SUI_SCRIPT="sui-exec"
  WORKDIR_NAME=""
  ;;
esac

# Configuration files (often needed for sui CLI calls)
export NETWORK_CONFIG="$CONFIG_DATA_DIR/network.yaml"
export CLIENT_CONFIG="$CONFIG_DATA_DIR/client.yaml"

# This is the default repo for localnet/devnet/testnet/mainnet scripts.
# Normally $SUI_REPO_DIR will symlink to $SUI_REPO_DIR_DEFAULT
export SUI_REPO_DIR_DEFAULT="$WORKDIRS/$WORKDIR/sui-repo-default"

# This is the default config for localnet/devnet/testnet/mainnet scripts.
# Normally $CONFIG_DATA_DIR will symlink to CONFIG_DATA_DIR_DEFAULT
export CONFIG_DATA_DIR_DEFAULT="$WORKDIRS/$WORKDIR/config-default"

# Location for genesis data for "default" repo.
export DEFAULT_GENESIS_DATA_DIR="$SCRIPTS_DIR/genesis_data"

# Location for generated genesis data (on first start after set-sui-repo)
export GENERATED_GENESIS_DATA_DIR="$WORKDIRS/$WORKDIR/genesis-data"

# The two shims find in each $WORKDIR
export SUI_EXEC="$WORKDIRS/$WORKDIR/sui-exec"
export WORKDIR_EXEC="$WORKDIRS/$WORKDIR/workdir-exec"

# Where all the sui.log of the sui client go to die.
export SUI_CLIENT_LOG_DIR="$WORKDIRS/$WORKDIR/logs/sui.log"

# Control if network execution, interaction and
# publication are to be mock.
#
# Intended for limited CI tests (github action).
export SUI_BASE_NET_MOCK=false
export SUI_BASE_NET_MOCK_VER="sui 0.99.99-abcdef"
export SUI_BASE_NET_MOCK_PID="999999"

# Utility macro specific to calling the keytool in a safer manner
# such that no "key" information gets actually log.
export NOLOG_KEYTOOL_BIN="env RUST_LOG=OFF $SUI_BIN_DIR/sui keytool"

# Cleanup should be always called on outer script exit.
#
# It is assumed that a "trap cleanup EXIT" will be initialized
# by the outer script (the one who sources __globals.sh).

# Initialize an associative array to track whether the lock has been acquired for each _WORKDIR
export SUIBASE_CLI_LOCK_ACQUIRED_LOCALNET=0
export SUIBASE_CLI_LOCK_ACQUIRED_DEVNET=0
export SUIBASE_CLI_LOCK_ACQUIRED_TESTNET=0
export SUIBASE_CLI_LOCK_ACQUIRED_MAINNET=0
export SUIBASE_CLI_LOCK_ACQUIRED_CARGOBIN=0
export SUIBASE_CLI_LOCK_ACQUIRED_ACTIVE=0
export SUIBASE_CLI_LOCK_ACQUIRED_SUIBASE_DAEMON=0
export SUIBASE_CLI_LOCK_ACQUIRED_WALRUS=0
export SUIBASE_CLI_LOCK_DISABLED=0

# Disable CLI mutex mechanism.
#
# Useful for when a script is called by a script that already acquired the necessary lock.
# In that case, the child script should not use any of the mutex.
cli_mutex_disable() {
  SUIBASE_CLI_LOCK_DISABLED=1
}
export -f cli_mutex_disable

# Allow to disable a lock. Used for
cleanup() {
  # echo "Cleanup called"
  # Clear progress files created by this script.
  if [ "$SUIBASE_DAEMON_UPGRADING" == "true" ]; then
    rm -f /tmp/.suibase/suibase-daemon-upgrading >/dev/null 2>&1
  fi

  # Associative arrays are not working for the trap. bash limitation?
  # Did workaround by painfully defining variables for each workdir.
  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_ACTIVE" == "1" ]; then
    cli_mutex_release "active"
    SUIBASE_CLI_LOCK_ACQUIRED_ACTIVE=0
  fi

  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_LOCALNET" == "1" ]; then
    cli_mutex_release "localnet"
    SUIBASE_CLI_LOCK_ACQUIRED_LOCALNET=0
  fi

  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_DEVNET" == "1" ]; then
    cli_mutex_release "devnet"
    SUIBASE_CLI_LOCK_ACQUIRED_DEVNET=0
  fi

  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_TESTNET" == "1" ]; then
    cli_mutex_release "testnet"
    SUIBASE_CLI_LOCK_ACQUIRED_TESTNET=0
  fi

  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_MAINNET" == "1" ]; then
    cli_mutex_release "mainnet"
    SUIBASE_CLI_LOCK_ACQUIRED_MAINNET=0
  fi

  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_CARGOBIN" == "1" ]; then
    cli_mutex_release "cargobin"
    SUIBASE_CLI_LOCK_ACQUIRED_CARGOBIN=0
  fi

  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_SUIBASE_DAEMON" == "1" ]; then
    cli_mutex_release "suibase_daemon"
    SUIBASE_CLI_LOCK_ACQUIRED_SUIBASE_DAEMON=0
  fi

  if [ "$SUIBASE_CLI_LOCK_ACQUIRED_WALRUS" == "1" ]; then
    cli_mutex_release "walrus"
    SUIBASE_CLI_LOCK_ACQUIRED_WALRUS=0
  fi

}

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
info_exit() {
  echo "$*"
  exit 0
}
export -f info_exit

error_exit() {
  {
    echo_red "Error: "
    echo "$*"
  } 1>&2
  exit 1
}
export -f error_exit

setup_error() {
  {
    echo_red "Error: "
    echo "$*"
  } 1>&2
  exit 1
}
export -f setup_error

warn_user() { {
  echo_yellow "Warning: "
  echo "$*"
} 1>&2; }
export -f warn_user

version_greater_equal() {
  local _arg1 _arg2
  # Remove everything until first digit
  # Remove trailing "-build number" if specified.
  # Keep only major/minor, ignore patch if specified.
  # shellcheck disable=SC2001
  _arg1=$(echo "$1" | sed 's/^[^0-9]*//; s/-.*//; s/\(.*\)\.\(.*\)\..*/\1.\2/')
  # shellcheck disable=SC2001
  _arg2=$(echo "$2" | sed 's/^[^0-9]*//; s/-.*//; s/\(.*\)\.\(.*\)\..*/\1.\2/')
  # With --check=quiet, sort will return 0 if no sort needed. In other word,
  # the first argument is already greater or equal to the second.
  printf '%s\n%s\n' "$_arg2" "$_arg1" | sort --check=quiet --version-sort
}
export -f version_greater_equal

version_less_than() {
  if version_greater_equal "$1" "$2"; then
    false
    return
  fi
  true
  return
}
export -f version_less_than

beginswith() { case $2 in "$1"*) true ;; *) false ;; esac }
export -f beginswith

file_newer_than() {
  # Check if file $1 is newer than file $2
  # Return true on any error (assuming need attention).
  if [ ! -f "$1" ] || [ ! -f "$2" ]; then
    true
    return
  fi
  local _date1 _date2
  _date1=$(date -r "$1" +%s)
  _date2=$(date -r "$2" +%s)

  if [[ "$_date1" > "$_date2" ]]; then
    true
    return
  fi

  false
  return
}
export -f file_newer_than

is_installed() {
  # Check if a CLI command is installed.
  if ! command -v "$1" &>/dev/null; then
    # Try again with 'which' in case command -v is not working on that setup...
    if ! which "$1" &>/dev/null; then
      false
      return
    fi
  fi
  true
  return
}

exit_if_rust_build_deps_missing() {
  # Check if all rust/cargo building dependencies are installed.
  is_installed cmake || setup_error "Need to install cmake. See https://docs.sui.io/build/install#prerequisites"
  is_installed rustc || setup_error "Need to install rust. See https://docs.sui.io/build/install#prerequisites"
  is_installed cargo || setup_error "Need to install cargo. See https://docs.sui.io/build/install#prerequisites"

  # Verify Rust is recent enough.
  version_greater_equal "$(rustc --version)" "$MIN_RUST_VERSION" || setup_error "Upgrade rust to a more recent version"
}
export -f exit_if_rust_build_deps_missing

SUIBASE_LOCK_CMD=""
SUIBASE_LOCK_TRY_PARAMS=""
#SUIBASE_LOCK_BLOCKING_PARAMS=""

init_SUIBASE_LOCK_vars() {
  if [ -n "$SUIBASE_LOCK_CMD" ]; then
    return
  fi
  # Check what is available, prefer flock over lockf.
  # Reference: https://github.com/Freaky/run-one/blob/master/run-one
  if is_installed flock; then
    SUIBASE_LOCK_CMD="flock"
    SUIBASE_LOCK_TRY_PARAMS="-xn"
    #SUIBASE_LOCK_BLOCKING_PARAMS="-x"
  else
    if is_installed lockf; then
      SUIBASE_LOCK_CMD="lockf"
      SUIBASE_LOCK_TRY_PARAMS="-xt0"
      #SUIBASE_LOCK_BLOCKING_PARAMS="-x"
    else
      setup_error "Neither 'flock' or 'lockf' are available! Install one of them"
    fi
  fi
  # All locks should be located in same place.
  mkdir -p "$SUIBASE_TMP_DIR"
}

try_locked_command() {
  init_SUIBASE_LOCK_vars
  exec $SUIBASE_LOCK_CMD $SUIBASE_LOCK_TRY_PARAMS "$@"
}
export -f try_locked_command


cli_mutex_lock() {
  # mutex is re-entrant (only first call to cli_mutex_lock will acquire the lock).

  # Design choice:
  #  The mkdir+trap trick is admitedly not perfect, but we have to make
  #  a compromise here.
  #
  #  The more robust flock approach cannot be used because of complication
  #  with inherited file descriptors hold indefinitely by child processes
  #  (e.g. when starting background sui or sui-faucet)
  #
  #  The drawback of mkdir+trap are:
  #    - Stale lock if the script is killed *between* the mkdir creation success and
  #      the setting of the lock acquired flag ("<1 microseconds" window?).
  #    - Not working atomicly if /tmp is on NFS (unlikely!)
  #    - other scenario where the trap EXIT is not called?
  #
  # Worst case, because the lock is in /tmp, a stale lock can be workaround with a reboot...

  if [ "$SUIBASE_CLI_LOCK_DISABLED" == "1" ]; then
    return
  fi

  local _WORKDIR=$1

  if [ -z "$_WORKDIR" ]; then
    setup_error "Internal error. cli_mutex_lock called without a workdir specified"
  fi

  init_SUIBASE_LOCK_vars

  # Only acquire the lock if it has not been acquired yet
  local _IS_ACQUIRED=0
  case $_WORKDIR in
  localnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_LOCALNET
    ;;
  devnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_DEVNET
    ;;
  testnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_TESTNET
    ;;
  mainnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_MAINNET
    ;;
  cargobin)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_CARGOBIN
    ;;
  active)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_ACTIVE
    ;;
  suibase_daemon)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_SUIBASE_DAEMON
    ;;
  walrus)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_WALRUS
    ;;
  *)
    setup_error "Internal error. cli_mutex_lock called with an unknown workdir $_WORKDIR"
    ;;
  esac
  if [ "$_IS_ACQUIRED" == "1" ]; then
    return
  fi

  local _LOCKFILE="$SUIBASE_TMP_DIR/cli-$_WORKDIR.lock"

  # Block until getting the lock. The lock is released by the "trap cleanup EXIT".
  while ! mkdir "$_LOCKFILE" 2>/dev/null; do
    sleep 0.2
  done

  case $_WORKDIR in
  localnet)
    SUIBASE_CLI_LOCK_ACQUIRED_LOCALNET=1
    ;;
  devnet)
    SUIBASE_CLI_LOCK_ACQUIRED_DEVNET=1
    ;;
  testnet)
    SUIBASE_CLI_LOCK_ACQUIRED_TESTNET=1
    ;;
  mainnet)
    SUIBASE_CLI_LOCK_ACQUIRED_MAINNET=1
    ;;
  cargobin)
    SUIBASE_CLI_LOCK_ACQUIRED_CARGOBIN=1
    ;;
  active)
    SUIBASE_CLI_LOCK_ACQUIRED_ACTIVE=1
    ;;
  suibase_daemon)
    SUIBASE_CLI_LOCK_ACQUIRED_SUIBASE_DAEMON=1
    ;;
  walrus)
    SUIBASE_CLI_LOCK_ACQUIRED_WALRUS=1
    ;;
  *)
    setup_error "Internal error. cli_mutex_lock called with an unknown workdir $_WORKDIR"
    ;;
  esac
}
export -f cli_mutex_lock

cli_mutex_release()
{
  # Use with care, this will release the lock regardless of how many
  # times cli_mutex_lock() was called.
  #
  local _WORKDIR=$1
  if [ -z "$_WORKDIR" ]; then
    setup_error "Internal error. cli_mutex_release called without a workdir specified"
  fi

  local _IS_ACQUIRED=0
  case $_WORKDIR in
  localnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_LOCALNET
    ;;
  devnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_DEVNET
    ;;
  testnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_TESTNET
    ;;
  mainnet)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_MAINNET
    ;;
  cargobin)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_CARGOBIN
    ;;
  active)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_ACTIVE
    ;;
  suibase_daemon)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_SUIBASE_DAEMON
    ;;
  walrus)
    _IS_ACQUIRED=$SUIBASE_CLI_LOCK_ACQUIRED_WALRUS
    ;;
  *)
    setup_error "Internal error. cli_mutex_release called with an unknown workdir $_WORKDIR"
    ;;
  esac
  if [ "$_IS_ACQUIRED" == "0" ]; then
    return
  fi

  local _LOCKFILE="$SUIBASE_TMP_DIR/cli-$_WORKDIR.lock"
  rmdir "$_LOCKFILE" >/dev/null 2>&1

  # Mark the lock as released
  case $_WORKDIR in
  localnet)
    SUIBASE_CLI_LOCK_ACQUIRED_LOCALNET=0
    ;;
  devnet)
    SUIBASE_CLI_LOCK_ACQUIRED_DEVNET=0
    ;;
  testnet)
    SUIBASE_CLI_LOCK_ACQUIRED_TESTNET=0
    ;;
  mainnet)
    SUIBASE_CLI_LOCK_ACQUIRED_MAINNET=0
    ;;
  cargobin)
    SUIBASE_CLI_LOCK_ACQUIRED_CARGOBIN=0
    ;;
  active)
    SUIBASE_CLI_LOCK_ACQUIRED_ACTIVE=0
    ;;
  suibase_daemon)
    SUIBASE_CLI_LOCK_ACQUIRED_SUIBASE_DAEMON=0
    ;;
  walrus)
    SUIBASE_CLI_LOCK_ACQUIRED_WALRUS=0
    ;;
  esac

  #echo "Lock released for $_WORKDIR"
}
export -f cli_mutex_release

set_key_value() {
  local _WORKDIR=$1
  local _KEY=$2
  local _VALUE=$3
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
  mkdir -p "$WORKDIRS/$_WORKDIR/.state"
  echo "$_VALUE" >|"$WORKDIRS/$_WORKDIR/.state/$_KEY"
}
export -f set_key_value

get_key_value() {
  local _WORKDIR=$1
  local _KEY=$2
  # A key-value persisted in the workdir.
  # Return the string NULL on error or missing.
  if [ -z "$_KEY" ]; then
    setup_error "Can't retreive empty key"
  fi
  if [ ! -f "$WORKDIRS/$_WORKDIR/.state/$_KEY" ]; then
    echo "NULL"
    return
  fi

  local _VALUE
  _VALUE=$(cat "$WORKDIRS/$_WORKDIR/.state/$_KEY")

  if [ -z "$_VALUE" ]; then
    echo "NULL"
    return
  fi

  # Error
  echo "$_VALUE"
}
export -f get_key_value

del_key_value() {
  local _WORKDIR=$1
  local _KEY=$2
  if [ -z "$_KEY" ]; then
    setup_error "Can't delete an unspecified key"
  fi
  if [ -f "$WORKDIRS/$_WORKDIR/.state/$_KEY" ]; then
    rm -f "$WORKDIRS/$_WORKDIR/.state/$_KEY"
  fi
}
export -f del_key_value

# Now load all the $CFG_ variables from the suibase.yaml files.
# shellcheck source=SCRIPTDIR/__parse-yaml.sh
source "$SCRIPTS_DIR/common/__parse-yaml.sh"
update_suibase_yaml() {
  local _WORKDIR="$WORKDIR"

  if [ ! -d "$SCRIPTS_DIR/defaults/$_WORKDIR" ]; then
    # If the specified workdir name does not exists, fallback
    # to localnet defaults (least damageable possible).
    _WORKDIR="localnet"
  fi

  # Load all variables from the defaults/consts.yaml file.
  #
  # These are "global" level (not specific to a workdir) and not expected
  # to be modified by users.
  #
  # They exists to help maintenance and debugging.
  YAML_FILE="$SCRIPTS_DIR/defaults/consts.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval "$(parse_yaml "$YAML_FILE" "CFG_")"
  fi

  # Load the suibase defaults twice.
  #
  # First time with CFG_ prefix, the second time with CFGDEFAULT_
  #
  # This allow to detect if there was an override or not
  # from users (e.g. to re-assure the user in a message that
  # an override was applied).
  YAML_FILE="$SCRIPTS_DIR/defaults/$_WORKDIR/suibase.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval "$(parse_yaml "$YAML_FILE" "CFG_")"
    eval "$(parse_yaml "$YAML_FILE" "CFGDEFAULT_")"
  fi

  #
  # Load the users overrides with CFG_ prefix.
  #
  # The common users overrides are loaded first and the more
  # precise workdir overrides are applied last.
  YAML_FILE="$WORKDIRS/common/suibase.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval "$(parse_yaml "$YAML_FILE" "CFG_")"
  fi

  YAML_FILE="$WORKDIRS/$_WORKDIR/suibase.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval "$(parse_yaml "$YAML_FILE" "CFG_")"
  fi
}
export -f update_suibase_yaml

update_suibase_yaml

update_SUIBASE_VERSION_var() {
  # Best effort to add the build number to the version.
  # If no success, just use the hard coded major.minor.patch info.
  local _BUILD
  _BUILD=$(if cd "$SCRIPTS_DIR"; then git rev-parse --short HEAD; else echo "-"; fi)
  if [ -n "$_BUILD" ] && [ "$_BUILD" != "-" ]; then
    SUIBASE_VERSION="$SUIBASE_VERSION-$_BUILD"
  fi
}
export -f update_SUIBASE_VERSION_var

cd_sui_log_dir() {
  # Deprecate changing the directory.
  # This is no longer needed See https://github.com/ChainMovers/suibase/issues/112
  #if [ -d "$WORKDIRS/$WORKDIR" ]; then
  #  mkdir -p "$SUI_CLIENT_LOG_DIR"
  #  cd "$SUI_CLIENT_LOG_DIR" || setup_error "could not access [ $SUI_CLIENT_LOG_DIR ]"
  #fi
  :
}
export -f cd_sui_log_dir

# cd_sui_log_dir

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
      setup_error "Bad commit to github. Contact https://suibase.io devs ASAP to fix the release."
    fi
  fi

  # Mocking can be control with suibase.yaml.
  #
  # This is undocummented, for suibase devs only.
  if [ "$CFG_SUI_BASE_NET_MOCK" = "true" ]; then
    SUI_BASE_NET_MOCK=true
  fi

  # If mocking AND state is started, then simulate
  # that the process are already running.
  if $SUI_BASE_NET_MOCK; then
    local _USER_REQUEST
    _USER_REQUEST=$(get_key_value "$WORKDIR" "user_request")
    if [ "$_USER_REQUEST" = "start" ]; then
      export SUI_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
      export SUI_FAUCET_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
    fi
  fi
}
export -f update_SUI_BASE_NET_MOCK_var

update_SUI_BASE_NET_MOCK_var

check_yaml_parsed() {
  local _var_name="CFG_$1"
  # Will fail if either not set or empty string. Both
  # wrong in all cases when calling this function.
  if [ -z "${!_var_name}" ]; then
    setup_error "Missing [ $_var_name ] in suibase.yaml."
  fi

  _var_name="CFGDEFAULT_$1"
  if [ -z "${!_var_name}" ]; then
    setup_error "Missing [ $_var_name ] in *default* suibase.yaml"
  fi

}
export -f check_yaml_parsed

build_sui_repo_branch() {
  ALLOW_DOWNLOAD="$1"
  ALLOW_BINARY="$2"
  DISABLE_AVX="$3"
  DISABLE_AVX2="$4"
  USE_PRECOMPILED="$5"
  PASSTHRU_OPTIONS="$6"

  local _BUILD_DESC
  if [ "${CFG_network_type:?}" = "local" ]; then
    is_local=true
    _BUILD_DESC="binaries"
  else
    is_local=false
    _BUILD_DESC="client"
  fi

  # If there is no checkout, no build etc... then still want to display
  # some feedback of the branch/tag in use before exiting the function.
  local _FEEDBACK_BEFORE_RETURN=true

  # Verify Sui pre-requisites are installed.
  update_HOST_vars
  if [[ $HOST_PLATFORM == "Darwin" ]]; then
    is_installed brew || setup_error "Need to install brew. See https://docs.sui.io/build/install#prerequisites"
  fi

  is_installed curl || setup_error "Need to install curl. See https://docs.sui.io/build/install#prerequisites"
  is_installed git || setup_error "Need to install git. See https://docs.sui.io/build/install#prerequisites"
  is_installed lsof || setup_error "Need to install 'lsof'."

  local _IS_SET_SUI_REPO="false"
  if is_sui_repo_dir_override; then
    _IS_SET_SUI_REPO="true"
  fi

  if [ "$ALLOW_DOWNLOAD" = "false" ]; then
    if [ "$_IS_SET_SUI_REPO" = true ]; then
      echo "Skipping git clone/fetch/pull because set-sui-repo is set."
      if [ ! -d "$RESOLVED_SUI_REPO_DIR" ]; then
        echo "Error: repo not found at [$RESOLVED_SUI_REPO_DIR]"
        echo "Either create this repo, or revert $WORKDIR_NAME to work with"
        echo "the default repo by typing \"$WORKDIR_NAME set-sui-repo\"".
        exit
      fi
    else
      # At least one of ALLOW_DOWNLOAD, USE_PRECOMPILED or set-sui-repos must be true,
      # otherwise there is no way to have sui binaries installed!
      if [ "$USE_PRECOMPILED" = "false" ]; then
        setup_error "No way to download or build the binaries allowed. Check your suibase.yaml settings."
      fi
      # Make sure sui-repo-default and sui-repo link are properly set. These are needed only for
      # the "target" directory... the location for binaries installation.
      if [ ! -d "$SUI_REPO_DIR_DEFAULT" ]; then
        mkdir -p "$SUI_REPO_DIR_DEFAULT"
        set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT"
      elif [ ! -L "$SUI_REPO_DIR" ]; then
        set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT"
      fi
    fi
  else
    if [ "${CFG_default_repo_url:?}" != "${CFGDEFAULT_default_repo_url:?}" ] ||
      [ "${CFG_default_repo_branch:?}" != "${CFGDEFAULT_default_repo_branch:?}" ]; then
      echo "suibase.yaml: Using repo [ $CFG_default_repo_url ] branch [ $CFG_default_repo_branch ]"
    fi

    # Delete sui-repo-default when not fully initialized. Either a corruption or can
    # normally happen when the user switch the config to allow repo download and
    # previous config created sui-repos-default only for binary download.
    if [ -d "$SUI_REPO_DIR_DEFAULT" ]; then
      if [ ! -f "$SUI_REPO_DIR_DEFAULT/Cargo.toml" ] || [ ! -d "$SUI_REPO_DIR_DEFAULT/.git" ]; then
        rm -rf "$SUI_REPO_DIR_DEFAULT"
      fi
    fi

    # If not already done, initialize the default repo.
    # Add back the default sui-repo link in case it was deleted.
    if [ ! -d "$SUI_REPO_DIR_DEFAULT" ]; then
      git clone -b "$CFG_default_repo_branch" "$CFG_default_repo_url" "$SUI_REPO_DIR_DEFAULT" || setup_error "Failed cloning branch [$CFG_default_repo_branch] from [$CFG_default_repo_url]"
      set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT"
    elif [ ! -L "$SUI_REPO_DIR" ]; then
      set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT"
    fi

    # Force git reset  if this is the very first time cloning (cover for
    # some scenario where the user Ctrl-C in middle of initial git object
    # fetching).
    local _FORCE_GIT_RESET=false
    if [ ! -d "$SUI_REPO_DIR/target" ]; then
      _FORCE_GIT_RESET=true
    fi

    # Update sui devnet local repo (if needed)
    #(cd "$SUI_REPO_DIR" && git switch "$CFG_default_repo_branch" >/dev/null 2>&1)

    if [ "$USE_PRECOMPILED" = "false" ]; then
      local _BRANCH
      _BRANCH=$(cd "$SUI_REPO_DIR" && git branch --show-current)

      if [ "$_BRANCH" != "$CFG_default_repo_branch" ]; then
        (cd "$SUI_REPO_DIR" && git checkout -f "$CFG_default_repo_branch" >/dev/null 2>&1)
      fi

      (cd "$SUI_REPO_DIR" && git remote update >/dev/null 2>&1)
      V1=$(if cd "$SUI_REPO_DIR"; then git rev-parse HEAD; else setup_error "unexpected missing $SUI_REPO_DIR"; fi)
      V2=$(if cd "$SUI_REPO_DIR"; then git rev-parse '@{u}'; else setup_error "unexpected missing $SUI_REPO_DIR"; fi)
      if [ "$V1" != "$V2" ]; then
        _FORCE_GIT_RESET=true
      fi

      if $_FORCE_GIT_RESET; then
        # Does a bit more than needed, but should allow to recover
        # from most operator error...
        echo "Updating sui $WORKDIR in ~/suibase/workdirs/$WORKDIR/sui-config..."
        (cd "$SUI_REPO_DIR" && git fetch >/dev/null)
        (cd "$SUI_REPO_DIR" && git reset --hard origin/"$CFG_default_repo_branch" >/dev/null)
        (cd "$SUI_REPO_DIR" && git merge '@{u}' >/dev/null)
      fi
    fi
  fi


  if [ "$ALLOW_BINARY" = false ]; then
    return
  fi

  # Either download precompiled or build from source.
  local _DO_FINAL_SUI_SANITY_CHECK=false
  local _DO_FINAL_SUI_FAUCET_SANITY_CHECK=false

  local _PRECOMP_STATE
  _PRECOMP_STATE=$(get_key_value "$WORKDIR" "precompiled")

  if [ "$USE_PRECOMPILED" = "true" ]; then
    # Identify the latest remote tag with binaries.
    update_PRECOMP_REMOTE_var
    if [ "$PRECOMP_REMOTE" != "true" ]; then
      setup_error "Could not retreive latest precompiled binaries for this platform"
    fi
    # Download the latest binary asset for this host.
    #
    # It will be "installed" later after the matching repo
    # is initialized/updated.
    download_PRECOMP_REMOTE "$WORKDIR"

    if [ "$ALLOW_DOWNLOAD" = "true" ]; then
      # Sync local repo... if allowed.
      local _DETACHED_INFO
      _DETACHED_INFO=$(cd "$SUI_REPO_DIR" && git branch | grep detached)
      # Checkout if _DETACHED_INFO does NOT contain the PRECOMP_REMOTE_TAG_NAME substring.
      if [[ "$_DETACHED_INFO" != *"$PRECOMP_REMOTE_TAG_NAME"* ]]; then
        # Checkout the tag that match the precompiled binary.
        echo "Checkout for $WORKDIR from repo [$CFG_default_repo_url] tag [$PRECOMP_REMOTE_TAG_NAME]"
        (cd "$SUI_REPO_DIR_DEFAULT" && git fetch >/dev/null 2>&1)
        (cd "$SUI_REPO_DIR_DEFAULT" && git reset --hard origin/"$CFG_default_repo_branch" >/dev/null 2>&1)
        (cd "$SUI_REPO_DIR_DEFAULT" && git switch "$CFG_default_repo_branch" >/dev/null 2>&1)
        (cd "$SUI_REPO_DIR_DEFAULT" && git merge '@{u}' >/dev/null 2>&1)
        (cd "$SUI_REPO_DIR_DEFAULT" && git checkout "$PRECOMP_REMOTE_TAG_NAME" >/dev/null 2>&1)
        _FEEDBACK_BEFORE_RETURN=false
      fi
    fi

    # Install the precompiled binary.
    install_PRECOMP_REMOTE "$WORKDIR"
    _DO_FINAL_SUI_SANITY_CHECK=true
  else
    # Build from source.

    if [ ! -d "$SUI_REPO_DIR" ]; then
      # Help user doing things out-of-order (like trying to regen something not yet initialized?)
      echo
      echo "The Sui repo is not initialized."
      echo
      echo " Do one of the following:"
      echo "    $WORKDIR start (recommended)"
      echo "    $WORKDIR update"
      echo
      exit 1
    fi

    exit_if_rust_build_deps_missing
    local _IS_RELEASE_BUILD=false
    if [[ "${CFG_cargo_release:?}" == "true" ]] || [[ "$PASSTHRU_OPTIONS" == *"--release"* ]]; then
      _IS_RELEASE_BUILD=true
      # delete target/debug content to avoid confusion.
      # Will be restored later on success.
      rm -rf "$WORKDIRS/$_WORKDIR/sui-repo/target/debug/*"
    fi

    if [ "$_PRECOMP_STATE" != "NULL" ]; then
      if [ "$_IS_SET_SUI_REPO" = "false" ]; then
        # Precompile was used before, so cleanup first to avoid confusion.
        (if cd "$SUI_REPO_DIR"; then cargo clean; else setup_error "Unexpected missing $SUI_REPO_DIR"; fi)
      fi
      del_key_value "$WORKDIR" "precompiled"
      _PRECOMP_STATE=$(get_key_value "$WORKDIR" "precompiled")
      # Sanity test.
      if [ "$_PRECOMP_STATE" != "NULL" ]; then
        setup_error "Unexpected precompiled state [$_PRECOMP_STATE] (1)"
      fi
    fi

    if [ "$_IS_SET_SUI_REPO" = "true" ]; then
      echo "Building $WORKDIR $_BUILD_DESC at [$RESOLVED_SUI_REPO_DIR]"
    else
      echo "Building $WORKDIR $_BUILD_DESC from latest repo [$CFG_default_repo_url] branch [$CFG_default_repo_branch]"
    fi

    # TODO Could not get _RUST_TARGET_CPU to work... not used for now.
    local _RUST_TARGET
    if [ "$_IS_RELEASE_BUILD" = "true" ]; then
      # Set _RUST_TARGET to --release if not done in user provided options.
      if [[ "$PASSTHRU_OPTIONS" != *"--release"* ]]; then
        _RUST_TARGET="--release"
      fi
    fi

    local _RUST_TARGET_CPU
    if [ "$DISABLE_AVX" = "true" ]; then
      # shellcheck disable=SC2089 # Quotes are OK here.
      _RUST_TARGET_CPU="-C target-cpu=nehalem"
    elif [ "$DISABLE_AVX2" = "true" ]; then
      # shellcheck disable=SC2089 # Quotes are OK here.
      _RUST_TARGET_CPU="-C target-cpu=sandybridge"
    fi

    local _BIN_LIST
    if [ $is_local = true ]; then
      _BIN_LIST="--bin sui --bin sui-faucet"
      _DO_FINAL_SUI_FAUCET_SANITY_CHECK=true
    else
      _BIN_LIST="--bin sui"
    fi
    _DO_FINAL_SUI_SANITY_CHECK=true

    if [ -n "$PASSTHRU_OPTIONS" ]; then
      if [[ "$PASSTHRU_OPTIONS" == *"-p "* || "$PASSTHRU_OPTIONS" == *"--bin "* ]]; then
        # The caller wants to control what gets build, so clear _BIN_LIST.
        _BIN_LIST=""
        # Can't assume the caller is building these.
        _DO_FINAL_SUI_SANITY_CHECK=false
        _DO_FINAL_SUI_FAUCET_SANITY_CHECK=false
      fi
    fi

    unset RUSTFLAGS
    # shellcheck disable=SC2086,SC2090 # Not using quotes around $_RUST_XXXX vars is intended here.
    (if cd "$SUI_REPO_DIR"; then cargo build --locked $_RUST_TARGET $_BIN_LIST $PASSTHRU_OPTIONS; else setup_error "unexpected missing $SUI_REPO_DIR"; fi)

    # If the build was release, copy it where it is expected by the scripts (target/debug)
    # Although not really a debug build... it just does not matter (will be overwritten if the
    # user really care building a debug version).
    if [ "$_IS_RELEASE_BUILD" = "true" ]; then
      local _SRC_DIR="$SUI_REPO_DIR/target/release"
      local _DST_DIR="$SUI_REPO_DIR/target/debug"
      mkdir -p "$_DST_DIR"
      local _SRC_LIST=("sui" "sui-faucet")
      # Iterate the _SRC_LIST array and copy the minimally needed files.
      for _SRC_FILE in "${_SRC_LIST[@]}"; do
        if [ -f "$_SRC_DIR/$_SRC_FILE" ]; then
          cp "$_SRC_DIR/$_SRC_FILE" "$_DST_DIR/$_SRC_FILE"
        fi
      done
    fi
  fi

  # First sanity check depends if the binary is expected or not...
  if [ "$_DO_FINAL_SUI_SANITY_CHECK" = "true" ]; then
    # Error if the sui binary does not exists, while it is expected.
    if [ ! -f "$SUI_BIN_DIR/sui" ]; then
      setup_error "$SUI_BIN_DIR/sui binary not found"
    fi
  fi

  if [ "$_DO_FINAL_SUI_FAUCET_SANITY_CHECK" = "true" ]; then
    # Error if the sui-faucet binary does not exists, while it is expected.
    if [ ! -f "$SUI_BIN_DIR/sui-faucet" ]; then
      setup_error "$SUI_BIN_DIR/sui-faucet binary not found"
    fi
  fi

  # This second group of sanity checks are always done if the sui binary exists.
  if [ -f "$SUI_BIN_DIR/sui" ]; then
    update_SUI_VERSION_var

    # Check if sui is recent enough.
    version_greater_equal "$SUI_VERSION" "$MIN_SUI_VERSION" || setup_error "Sui binary version too old (not supported)"
  fi

  # Syncronize .state/precompiled with the .cache binary filepath.
  local _DELETE_PRECOMP_STATE="false"
  if [ "$USE_PRECOMPILED" = "false" ]; then
    if [ "$_PRECOMP_STATE" != "NULL" ]; then
      _DELETE_PRECOMP_STATE="true"
    fi
  else
    if [ -z "$PRECOMP_REMOTE_DOWNLOAD_DIR" ]; then
      # That should never happen, but check just in case.
      _DELETE_PRECOMP_STATE="true"
      warn_user "Can't sync precompiled state with unavailable download info".
    else
      if [ "$_PRECOMP_STATE" != "$PRECOMP_REMOTE_DOWNLOAD_DIR" ]; then
        set_key_value "$WORKDIR" "precompiled" "$PRECOMP_REMOTE_DOWNLOAD_DIR"
      fi
    fi
  fi

  if [ "$_DELETE_PRECOMP_STATE" = "true" ]; then
    del_key_value "$WORKDIR" "precompiled"
    _PRECOMP_STATE=$(get_key_value "$WORKDIR" "precompiled")
    # Sanity test.
    if [ "$_PRECOMP_STATE" != "NULL" ]; then
      setup_error "Unexpected precompiled state [$_PRECOMP_STATE] (2)"
    fi
  fi

  # Use key-value states to remember which configuration was used
  # to build the binaries.
  local _REPO_URL_STATE=$(get_key_value "$WORKDIR" "repo_url")
  local _REPO_URL_EXPECTED
  if [ "$_IS_SET_SUI_REPO" = "true" ]; then
    _REPO_URL_EXPECTED="$RESOLVED_SUI_REPO_DIR"
  else
    _REPO_URL_EXPECTED="${CFG_default_repo_url:?}"
  fi
  if [ "$_REPO_URL_STATE" != "$_REPO_URL_EXPECTED" ]; then
    set_key_value "$WORKDIR" "repo_url" "$_REPO_URL_EXPECTED"
  fi

  local _REPO_BRANCH_STATE=$(get_key_value "$WORKDIR" "repo_branch")
  local _REPO_BRANCH_EXPECTED
  if [ "$_IS_SET_SUI_REPO" = "true" ]; then
    _REPO_BRANCH_EXPECTED="$RESOLVED_SUI_REPO_DIR"
  else
    _REPO_BRANCH_EXPECTED="${CFG_default_repo_branch:?}"
  fi
  if [ "$_REPO_BRANCH_STATE" != "$_REPO_BRANCH_EXPECTED" ]; then
    set_key_value "$WORKDIR" "repo_branch" "$_REPO_BRANCH_EXPECTED"
  fi

  # Help user by reminding the origin of the binaries.
  if [ "$_FEEDBACK_BEFORE_RETURN" = "true" ]; then
    if [ "$USE_PRECOMPILED" = "true" ]; then
      #echo "Using precompiled binaries from repo [$CFG_default_repo_url] tag [$PRECOMP_REMOTE_TAG_NAME]"
      echo "Using precompiled sui binaries [$PRECOMP_REMOTE_TAG_NAME]"
    else
      if [ "$_IS_SET_SUI_REPO" = true ]; then
        echo "Using binaries built from repo [$RESOLVED_SUI_REPO_DIR]"
      else
        echo "Using binaries built from latest repo [$CFG_default_repo_url] branch [$CFG_default_repo_branch]"
      fi
    fi
  fi
}
export -f build_sui_repo_branch

exit_if_not_installed() {
  # Help the user that did not even do the installation of the symlinks
  # and is trying instead to call directly from "~/suibase/scripts"
  # (which will cause some trouble with some script).
  case "$SCRIPT_NAME" in
  "asui" | "lsui" | "csui" | "dsui" | "tsui" | "localnet" | "devnet" | "testnet" | "mainnet" | "workdirs")
    if [ ! -L "$LOCAL_BIN_DIR/$SCRIPT_NAME" ]; then
      echo
      echo "Some suibase files are missing. The installation was"
      echo "either not done or failed."
      echo
      echo "Run ~/suibase/install again to fix this."
      echo
      exit 1
    fi
    ;;
  *) ;;
  esac

  # TODO Test suibase on $PATH is fine.
}
export -f exit_if_not_installed

exit_if_workdir_not_ok() {
  # This is a common "operator" error (not doing command in right order).
  if ! is_workdir_ok; then
    if [ "$WORKDIR" = "cargobin" ]; then
      exit_if_sui_binary_not_ok # Point to a higher problem (as needed).
      echo "cargobin workdir not initialized"
      echo
      echo "Please run ~/suibase/.install again to detect"
      echo "the ~/.cargo/bin/sui and create the cargobin workdir."
      echo
      echo "It is safe to re-run ~/suibase/.install when suibase"
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
  if [ ! -f "$SUI_BIN_DIR/sui" ]; then
    if [ "$WORKDIR" = "cargobin" ]; then
      echo "The $HOME/.cargo/bin/sui was not found."
      echo "Follow Mysten Lab procedure to install it:"
      echo " https://docs.sui.io/build/install#install-sui-binaries"
    else
      echo
      echo "The sui binary for $WORKDIR was not found."
      echo
      echo " Do one of the following to install it:"
      echo "    $WORKDIR start"
      echo "    $WORKDIR update"
      echo
    fi
    exit 1
  fi

  # Sometimes the binary are ok, but not the config (may happen when the
  # localnet config directory is safely wipe out on set-sui-repo transitions).
  if [ "$CFG_network_type" = "local" ]; then
    if [ ! -f "$CLIENT_CONFIG" ]; then
      echo
      echo "The localnet need to be regenerated."
      echo
      echo " Do one of the following:"
      echo "    $WORKDIR regen (recommended)"
      echo "    $WORKDIR update"
      echo
      exit 1
    fi

    update_SUI_VERSION_var # Requires $SUI_BIN_DIR/sui (was verified above)
    if version_greater_equal "$SUI_VERSION" "0.27"; then
      if [ ! -f "$SUI_BIN_DIR/sui-faucet" ]; then
        echo
        echo "The sui-faucet binary for $WORKDIR was not found."
        echo
        echo " Do one of the following to install it:"
        echo "    $WORKDIR start"
        echo "    $WORKDIR update"
        echo
        exit 1
      fi
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
    false
    return
  fi

  # Get the version, but in a way that would not exit on failure.
  # Deprecated cd_sui_log_dir
  local _SUI_VERSION_ATTEMPT
  _SUI_VERSION_ATTEMPT=$($SUI_BIN_ENV "$SUI_BIN_DIR/sui" --version)
  # TODO test here what would really happen on corrupted binary...
  if [ -z "$_SUI_VERSION_ATTEMPT" ]; then
    false
    return
  fi

  if [ "$CFG_network_type" = "local" ]; then
    if version_greater_equal "$_SUI_VERSION_ATTEMPT" "0.27"; then
      if [ ! -f "$SUI_BIN_DIR/sui-faucet" ]; then
        false
        return
      fi
    fi

    if [ ! -f "$NETWORK_CONFIG" ] || [ ! -f "$CLIENT_CONFIG" ]; then
      false
      return
    fi
  fi

  true
  return
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
    # Special case because no repo etc... (later to be handled better by suibase.yaml)
    # Just check for the basic.
    if [ ! -f "$HOME/.cargo/bin/sui" ]; then
      setup_error "This script is for user who choose to install ~/.cargo/bin/sui. You do not have it installed."
    fi

    if [ ! -d "$WORKDIRS" ]; then
      setup_error "$WORKDIRS missing. Please run '~/suibase/install' to repair"
    fi

    if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
      setup_error "$WORKDIRS/$WORKDIR missing. Please run '~/suibase/install' to repair"
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
    false
    return
  fi

  if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
    false
    return
  fi

  if [ ! -f "$WORKDIRS/$WORKDIR/sui-exec" ] ||
    [ ! -f "$WORKDIRS/$WORKDIR/workdir-exec" ]; then
    false
    return
  fi

  if [ ! -L "$WORKDIRS/$WORKDIR/config" ]; then
    false
    return
  fi

  true
  return
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
          ((i = 1))
          while IFS= read -r line; do
            if ! $_FIRST_LINE; then echo ","; else _FIRST_LINE=false; fi
            echo -n "\"sb-$i-$scheme\": { \"address\": \"$line\" }"
            ((i++))
          done < <(printf '%s\n' "$_LINES")
        done
        echo
        echo "}}"
      } >>"$WORKDIRS/$WORKDIR_PARAM/.state/dns"
    fi
  fi
}
export -f create_state_dns_as_needed

# Load the sui.aliases file into arrays.
#
# Allows access of every fields for every alias,
# using only an index.
#
# Valid only if SUI_ALIASES_SIZE > 0
#
# Index goes from 0 to SUI_ALIASES_SIZE-1
export SUI_ALIASES_SIZE
export SUI_ALIASES_NAME
export SUI_ALIASES_ADDRESS
export SUI_ALIASES_KEY_SCHEME
export SUI_ALIASES_PEER_ID
export SUI_ALIASES_PUBLIC_KEY
export SUI_ALIASES_FLAG

# Numbers produced while building the array
export SUI_ALIASES_MAX_INDEX_FOUND
export SUI_ALIASES_EMPTY_ALIAS_FOUND

update_sui_aliases_arrays() {
  SUI_ALIASES_SIZE=0
  SUI_ALIASES_NAME=()
  SUI_ALIASES_ADDRESS=()
  SUI_ALIASES_KEY_SCHEME=()
  SUI_ALIASES_PEER_ID=()
  SUI_ALIASES_PUBLIC_KEY=()
  SUI_ALIASES_FLAG=()
  SUI_ALIASES_MAX_INDEX_FOUND=0
  SUI_ALIASES_EMPTY_ALIAS_FOUND=0

  # "keytool list" output is a JSON array of elements looking like this:
  #
  #[
  #  {
  #    "alias": "priceless-opal",
  #    "suiAddress": "0xef6e9dd8f30dea802e0474a7996e5c772c581cc1adee45afb660f15a081d1c49",
  #    "publicBase64Key": "AgPe4JWk+Zledatd8nbMbWTNh1sqHU/0Dy9zn9S6FVQIIQ==",
  #    "keyScheme": "secp256r1",
  #    "flag": 2,
  #    "peerId": null
  #  },
  #  {
  #    "alias": "chaotic-diamond",
  #    "suiAddress": "0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462",
  #    "publicBase64Key": "ALSfnL+vbyJ55c0rCuR08k8AoYxS7o4xAyaQ1Lmw977B",
  #    "keyScheme": "ed25519",
  #    "flag": 0,
  #    "peerId": "b49f9cbfaf6f2279e5cd2b0ae474f24f00a18c52ee8e31032690d4b9b0f7bec1"
  #  }
  #  ... more elements...
  #]

  local _KEYTOOL_LIST
  local _KEYTOOL_EXEC
  _KEYTOOL_EXEC="env RUST_LOG=OFF $SUI_SCRIPT keytool"
  _KEYTOOL_LIST=$($_KEYTOOL_EXEC --json list 2>&1)

  # Iterate over every lines of _KEYTOOL_LIST
  local _INSIDE_BLOCK=false
  local _ALIAS _SUI_ADDRESS _KEY_SCHEME _PEER_ID _PUBLIC_KEY _FLAG

  # Trick to read even if last line does not have EOL:
  # https://stackoverflow.com/questions/12916352/shell-script-read-missing-last-line
  while IFS= read -r line || [ -n "$line" ]; do
    # Check if the line contains an opening curly bracket
    if [[ $line == *"{"* ]]; then
      _INSIDE_BLOCK=true
      unset _ALIAS _SUI_ADDRESS _KEY_SCHEME _PEER_ID _PUBLIC_KEY _FLAG
    fi

    # Check if the line contains a closing curly bracket
    if [[ $line == *"}"* ]]; then
      _INSIDE_BLOCK=false
      if [ -n "$_SUI_ADDRESS" ] && [ -n "$_KEY_SCHEME" ] && [ -n "$_PUBLIC_KEY" ]; then
        # Add the fields to the arrays.
        SUI_ALIASES_NAME+=("$_ALIAS")
        SUI_ALIASES_ADDRESS+=("$_SUI_ADDRESS")
        SUI_ALIASES_KEY_SCHEME+=("$_KEY_SCHEME")
        SUI_ALIASES_PEER_ID+=("$_PEER_ID")
        SUI_ALIASES_PUBLIC_KEY+=("$_PUBLIC_KEY")
        SUI_ALIASES_FLAG+=("$_FLAG")
        ((SUI_ALIASES_SIZE++))
      fi
    fi

    # Only process lines inside a block
    if $_INSIDE_BLOCK; then
      # Read the fields.
      if [[ $line == *"\"alias\":"* ]]; then
        _ALIAS=$(echo "$line" | awk -F'"' '{print $4}')
        if [ -z "$_ALIAS" ]; then
          ((SUI_ALIASES_EMPTY_ALIAS_FOUND++))
        else
          # Check if the alias string is the pattern alias-<number>.
          # Example: "alias-1", "alias-2", etc.
          if [[ $_ALIAS == "alias-"* ]]; then
            # Extract the number from the alias string.
            local _NUMBER
            _NUMBER=$(echo "$_ALIAS" | awk -F'-' '{print $2}')
            # Check if the number is a valid integer.
            if [[ $_NUMBER =~ ^[0-9]+$ ]]; then
              if [ "$_NUMBER" -gt "$SUI_ALIASES_MAX_INDEX_FOUND" ]; then
                SUI_ALIASES_MAX_INDEX_FOUND=$_NUMBER
              fi
            fi
          fi
        fi
      elif [[ $line == *"\"suiAddress\":"* ]]; then
        _SUI_ADDRESS=$(echo "$line" | awk -F'"' '{print $4}')
      elif [[ $line == *"\"keyScheme\":"* ]]; then
        _KEY_SCHEME=$(echo "$line" | awk -F'"' '{print $4}')
      elif [[ $line == *"\"peerId\":"* ]]; then
        _PEER_ID=$(echo "$line" | awk -F'"' '{print $4}')
      elif [[ $line == *"\"publicBase64Key\":"* ]]; then
        _PUBLIC_KEY=$(echo "$line" | awk -F'"' '{print $4}')
      elif [[ $line == *"\"flag\":"* ]]; then
        _FLAG=$(echo "$line" | awk -F': ' '{print $2}' | awk -F',' '{print $1}')
      fi
    fi
  done <<<"$_KEYTOOL_LIST"
}
export update_sui_aliases_arrays

adjust_sui_aliases() {
  local _WORKDIR_PARAM="$1"

  if [[ "${CFG_auto_key_generation:?}" == 'false' ]]; then
    return
  fi

  # Update sui.aliases with deterministic names for key created by Suibase.
  #
  # sui.aliases is located at $WORKDIRS/$WORKDIR_PARAM/config/sui.aliases
  #
  # The "suiAddress" of auto-created address are in the co-located recovery.txt file.
  #
  # Use the command "keytool list" to iterate and get all the fields for every alias.
  #
  # What does that function do?
  # ===========================
  # Iterate each "alias" and extract its corresponding "suiAddress" and "keyScheme" fields.
  #
  # If the "suiAddress" exists in the recovery.txt file, then replace the "alias" field with
  # a deterministic name based on a counter of how many alias of a given keyScheme type
  # were found to exists up to now.
  #
  # The derministic name is "sb-<count>-<keyScheme>" where <count> is the current one-base
  # counter for the given "keyScheme" type.
  #
  # If the "suiAddress" does not exists in the recovery.txt file, then the corresponding
  # "alias" is left unchanged.
  #
  # Implemented using a map to track the distinct "keyScheme" that are identified
  # along the way.
  #
  # What about address coming from add_private_keys in suibase.yaml?
  # ================================================================
  # They will be named alias-<number> in the order that they are listed by "keytool list".
  #

  local _SUI_ALIASES="$WORKDIRS/$_WORKDIR_PARAM/config/sui.aliases"
  local _RECOVERY_TXT="$WORKDIRS/$_WORKDIR_PARAM/config/recovery.txt"
  local _TEMP_FILE="$WORKDIRS/$_WORKDIR_PARAM/config/sui.aliases.tmp"

  rm -f "$_TEMP_FILE"

  if [ ! -f "$_SUI_ALIASES" ] || [ ! -f "$_RECOVERY_TXT" ]; then
    return
  fi

  # Process the sui.aliases file only if "sb-1-ed25519" is not found.
  # This is to avoid to wasting time for when the renaming was already done.
  if grep -q "sb-1-ed25519" "$_SUI_ALIASES"; then
    return
  fi

  update_sui_aliases_arrays

  if [ $SUI_ALIASES_SIZE -eq 0 ]; then
    return
  fi

  # Do the equivalent of readarray.
  # https://stackoverflow.com/questions/30988586/creating-an-array-from-a-text-file-in-bash
  # (Note: readarray is not available on older bash/MacOS)
  recovery=()
  while IFS= read -r line || [[ "$line" ]]; do
    recovery+=("$line")
  done <"$_RECOVERY_TXT"

  # Sanity test, and just do nothing if did not work (the binary might not
  # be built, in which case the user will likely try again).
  if [ ${#recovery[@]} -eq 0 ]; then
    return
  fi

  # Declare a counter for each keyScheme type
  # This is a hard-coded hashmap from key to an integer (counter).
  # First declare an array of key : "ed25519", "secp256k1", "secp256r1"
  local _KEY_SCHEME_ARRAY=("ed25519" "secp256k1" "secp256r1")
  # Now declare an array of counter : 0, 0, 0
  local _COUNTER_ARRAY=(0 0 0)

  local _ALIAS
  local _PUB_64_KEY
  local _KEY_SCHEME
  local _NEXT_INDEX
  _NEXT_INDEX=$((SUI_ALIASES_MAX_INDEX_FOUND + 1))

  # Important: This constant must match the logic done in init_workdir when auto-generating the addresses.
  local _AUTO_CREATED_PER_SCHEME=5

  # Work with a temporary file until success.
  \cp "$_SUI_ALIASES" "$_TEMP_FILE"

  # Iterate index from 0 to SUI_ALIASES_SIZE-1
  #
  # Each iteration is for a unique "alias" and "suiAddress"
  #
  # Trick to read even if last line does not have EOL:
  # https://stackoverflow.com/questions/12916352/shell-script-read-missing-last-line
  for ((i = 0; i < SUI_ALIASES_SIZE; i++)); do
    # Read the fields.
    _ALIAS=${SUI_ALIASES_NAME[$i]}
    _SUI_ADDRESS=${SUI_ALIASES_ADDRESS[$i]}
    _KEY_SCHEME=${SUI_ALIASES_KEY_SCHEME[$i]}
    _PUB64_KEY=${SUI_ALIASES_PUBLIC_KEY[$i]}

    if [ -n "$_SUI_ADDRESS" ] && [ -n "$_KEY_SCHEME" ]; then
      # echo "$_ALIAS $_PUB64_KEY"
      if [ -z "$_ALIAS" ]; then
        # Add a comma if there is at least one existing entry in the JSON array.
        if grep -q "}" "$_TEMP_FILE"; then
          # Remove the last "]" in $_TEMP_FILE. We will add it back later.
          sed -i.bak -e 's/]/ /' "$_TEMP_FILE" &&
            rm "$_TEMP_FILE.bak"

          # Replace the last line with "}" with "}," in $_TEMP_FILE.
          lines=()
          while IFS= read -r line; do
            lines+=("$line")
          done <"$_TEMP_FILE"

          # Iterate over the lines in reverse order
          for ((idx = ${#lines[@]} - 1; idx >= 0; idx--)); do
            # If the line contains "}", replace "}" with "}," and break the loop
            if [[ ${lines[idx]} == *"}"* ]]; then
              lines[idx]="${lines[idx]%\}}}",
              break
            fi
          done

          # Write the lines back to the file
          for line in "${lines[@]}"; do
            echo "$line"
          done >"$_TEMP_FILE"

          # Append "  ]" at the end of $_TEMP_FILE.
          echo "  ]" >>"$_TEMP_FILE"
        fi

        # Append _NEXT_INDEX to $_ALIAS and increment _NEXT_INDEX
        _ALIAS="alias-$_NEXT_INDEX"
        _NEXT_INDEX=$((1 + _NEXT_INDEX))

        # Append a new entry at the end of the JSON array in sui.aliases.
        #
        # Example of sui.aliases file:
        # [{
        #   "alias": "sb-2-secp256r1",
        #   "public_key_base64": "AgIrMYwPbHFcj+kR4dbn0bkzU82fGQfw4QzJjhWGAUd4zQ=="
        #  },
        #  {
        #    "alias": "sb-1-secp256r1",
        #    "public_key_base64": "AgPe4JWk+Zledatd8nbMbWTNh1sqHU/0Dy9zn9S6FVQIIQ=="
        #  }
        # ]
        #
        # The alias is given by $_ALIAS and the public_key_base64 is given by $_PUB_64_KEY.
        # Each field should be on its own line and indented as shown in the example above.
        local _NEW_ENTRY="{\n"
        _NEW_ENTRY+="    \"alias\": \"$_ALIAS\",\n"
        _NEW_ENTRY+="    \"public_key_base64\": \"$_PUB64_KEY\"\n"
        _NEW_ENTRY+="  }\n"

        # Append the new entry on the line before the "]"
        sed -i.bak -e "s/]/$_NEW_ENTRY]/" "$_TEMP_FILE" &&
          rm "$_TEMP_FILE.bak"
      else
        # Check if the suiAddress exists in the recovery.txt file
        for value in "${recovery[@]}"; do
          if [[ $value == *$_SUI_ADDRESS* ]]; then
            # echo "Found $_SUI_ADDRESS in recovery.txt"
            # Increment the counter for this $_KEY_SCHEME
            # Iterate _KEY_SCHEME_ARRAY to find the index to increment in _COUNTER_ARRAY
            local _KEY_IDX=0
            for ((idx = 0; idx < ${#_KEY_SCHEME_ARRAY[@]}; idx++)); do
              if [ "${_KEY_SCHEME_ARRAY[$idx]}" = "$_KEY_SCHEME" ]; then
                # Increment the counter for this $_KEY_SCHEME
                ((_COUNTER_ARRAY[idx]++))
                _KEY_IDX=$idx
                break
              fi
            done

            if [[ -n ${_COUNTER_ARRAY[$_KEY_IDX]} && ${_COUNTER_ARRAY[$_KEY_IDX]} =~ ^[0-9]+$ ]]; then
              # Replace the alias field with a deterministic name in the temporary file.
              local _SEARCH_STRING="$_ALIAS"
              local _ALIAS_NUMBER=$((1 + _AUTO_CREATED_PER_SCHEME - ${_COUNTER_ARRAY[$_KEY_IDX]}))
              local _REPLACE_STRING="sb-$_ALIAS_NUMBER-$_KEY_SCHEME"
              sed -i.bak -e "s/$_SEARCH_STRING/$_REPLACE_STRING/g" \
                "$_TEMP_FILE" &&
                rm "$_TEMP_FILE.bak"

              break
            else
              echo "Unexpected counter value for $_KEY_SCHEME"
            fi
          fi
        done
      fi
    fi
  done

  # Replace the original file with the temporary file
  mv "$_TEMP_FILE" "$_SUI_ALIASES"
}
export -f adjust_sui_aliases

create_state_links_as_needed() {
  WORKDIR_PARAM="$1"

  if [ "$WORKDIR_PARAM" = "cargobin" ]; then
    return
  fi

  # Create/repair (if possible)
  if [ ! -d "$WORKDIRS/$WORKDIR_PARAM/.state" ]; then
    mkdir -p "$WORKDIRS/$WORKDIR_PARAM/.state"
  fi

  # Take the user suibase.yaml and create the .state/links
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
  if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/.state/links" ] || file_newer_than "$WORKDIRS/$WORKDIR_PARAM/suibase.yaml" "$WORKDIRS/$WORKDIR_PARAM/.state/links"; then
    # parse_yaml ~/suibase/scripts/defaults/testnet/suibase.yaml;
    check_yaml_parsed links_

    ((_n_links = 0))
    for _links in ${CFG_links_:?}; do
      ((_n_links++))
    done

    if [ $_n_links -eq 0 ]; then
      setup_error "No links found in $WORKDIRS/$WORKDIR_PARAM/config/suibase.yaml"
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
      ((_i = 0))
      for _links in ${CFG_links_:?}; do
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
        ((_i++))
        echo -n "    }" # close link
      done
      echo
      echo "  ]" # close links
      echo "}"
    } >"$WORKDIRS/$WORKDIR_PARAM/.state/links.tmp"
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
    set_key_value "$WORKDIR_PARAM" "user_request" "stop"
  fi

  if [ "$WORKDIR_PARAM" != "active" ]; then
    if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/.state/name" ] ||
      [ "$(get_key_value "$WORKDIR_PARAM" "name")" != "$WORKDIR_PARAM" ]; then
      set_key_value "$WORKDIR_PARAM" "name" "$WORKDIR_PARAM"
    fi
  fi

  create_state_dns_as_needed "$WORKDIR_PARAM"
  create_state_links_as_needed "$WORKDIR_PARAM"
}
export -f create_state_as_needed

cleanup_cache_as_needed() {
  WORKDIR_PARAM="$1"

  # Create/repair
  if [ ! -d "$WORKDIRS/$WORKDIR_PARAM/.cache" ]; then
    mkdir -p "$WORKDIRS/$WORKDIR_PARAM/.cache"
    return
  fi

  if [ "$WORKDIR_PARAM" = "active" ]; then
    return
  fi

  # Only keep last 2 releases for each branch.
  local _PRECOMP_DOWNLOAD="$WORKDIRS/$WORKDIR_PARAM/.cache/precompiled_downloads"
  if [ -d "$_PRECOMP_DOWNLOAD" ]; then
    # Delete recursively every directory and files in $_PRECOMP_DOWNLOAD with a name
    # that does not match _BRANCH.
    local _BRANCH="${CFG_default_repo_branch:?}"
    for item in "$_PRECOMP_DOWNLOAD"/*; do
      if [ -z "$item" ] || [ "$item" = "." ] || [ "$item" = ".." ] || [ "$item" = "/" ]; then
        continue
      fi
      if [ -d "$item" ]; then
        if [ "$(basename "$item")" != "$_BRANCH" ]; then
          rm -rf "$item"
        else
          # Keep in the cache only the last 2 releases files and latest untar directories (up to 4 items),
          # delete all the rest.
          local _RELEASES
          # shellcheck disable=SC2012 # ls -1 is safe here. find is more risky for portability.
          _RELEASES=$(ls -1 "$item" | sort -r)
          local _KEEP=4
          for release in $_RELEASES; do
            if [ -z "$release" ] || [ "$release" = "." ] || [ "$release" = ".." ] || [ "$release" = "/" ]; then
              continue
            fi
            if [ $_KEEP -gt 0 ]; then
              ((_KEEP--))
            else
              # shellcheck disable=SC2115 # $item and $release validated to not be empty string.
              rm -rf "$item/$release"
            fi
          done
        fi
      fi
    done
  fi
}
export -f cleanup_cache_as_needed

repair_walrus_config_as_needed() {
  WORKDIR_PARAM="$1"

  if [ "$WORKDIR_PARAM" = "cargobin" ] || [ "$WORKDIR_PARAM" = "active" ]; then
    return
  fi

  # Replace $HOME string in client_config.yaml and sites-config.yaml with
  #the actual home directory.
  local _CONFIG_FILES=("client_config.yaml" "sites-config.yaml")

  for _CONFIG_FILE in "${_CONFIG_FILES[@]}"; do
    local _CONFIG_PATH="$WORKDIRS/$WORKDIR_PARAM/config/$_CONFIG_FILE"

    # Only process the file if it exists
    if [ -f "$_CONFIG_PATH" ]; then
      # Check if the file contains $HOME/ references
      if grep -q "\\\$HOME/" "$_CONFIG_PATH"; then
        # Create a temporary file for the replacement
        local _TEMP_FILE=$(mktemp)

        # Replace $HOME/ with the actual home directory path
        sed "s|\\\$HOME/|$HOME/|g" "$_CONFIG_PATH" > "$_TEMP_FILE"

        # Move the temporary file back to the original location
        mv "$_TEMP_FILE" "$_CONFIG_PATH"

        echo "Replaced \$HOME/ references in $_CONFIG_FILE with actual home directory"
      fi
    fi
  done
}
export -f repair_walrus_config_as_needed

repair_workdir_as_needed() {
  WORKDIR_PARAM="$1"

  if [ ! -d "$WORKDIRS" ]; then
    if ! mkdir -p "$WORKDIRS"; then
      setup_error "Unable to create $WORKDIRS"
    fi
  fi

  if [ ! -d "$SUIBASE_BIN_DIR" ]; then
    if ! mkdir -p "$SUIBASE_BIN_DIR"; then
      setup_error "Unable to create $SUIBASE_BIN_DIR"
    fi
  fi

  if [ ! -d "$SUIBASE_LOGS_DIR" ]; then
    if ! mkdir -p "$SUIBASE_LOGS_DIR"; then
      setup_error "Unable to create $SUIBASE_LOGS_DIR"
    fi
  fi

  if [ ! -d "$SUIBASE_TMP_DIR" ]; then
    if ! mkdir -p "$SUIBASE_TMP_DIR"; then
      setup_error "Unable to create $SUIBASE_TMP_DIR"
    fi
  fi

  if [ "$WORKDIR_PARAM" = "active" ]; then
    update_ACTIVE_WORKDIR_var
    if [ -z "$ACTIVE_WORKDIR" ] || [ ! -d "$WORKDIRS/$ACTIVE_WORKDIR" ]; then
      # Do not create an "active" directory, but...
      return
    fi
    # ... keep going to repair if pointing to a valid directory.
    WORKDIR_PARAM="$ACTIVE_WORKDIR"
  else
    if [ -d "$SCRIPTS_DIR/templates/$WORKDIR_PARAM" ]; then
      # Copy/fix recursively without overwriting existing files (-n)
      mkdir -p "$WORKDIRS/$WORKDIR_PARAM"
      cp -rn "$SCRIPTS_DIR/templates/$WORKDIR_PARAM"/* "$WORKDIRS/$WORKDIR_PARAM/" 2>/dev/null

      # As needed, initialize common suibase.yaml from template as well.
      # This is useful for when tests/run-all.sh adds variables
      # to the common/suibase.yaml (e.g. github_token).
      if [ ! -f "$WORKDIRS/common/suibase.yaml" ] && [ -f "$SCRIPTS_DIR/templates/common/suibase.yaml" ]; then
        #mkdir -p "$SCRIPTS_DIR/templates/common/"
        mkdir -p "$WORKDIRS/common"
        cp "$SCRIPTS_DIR/templates/common/suibase.yaml" "$WORKDIRS/common/suibase.yaml"
      fi

      repair_walrus_config_as_needed "$WORKDIR_PARAM"
    fi
    create_active_symlink_as_needed "$WORKDIR_PARAM"
  fi

  create_exec_shims_as_needed "$WORKDIR_PARAM"
  create_state_as_needed "$WORKDIR_PARAM"
  cleanup_cache_as_needed "$WORKDIR_PARAM"

  if [ "$WORKDIR_PARAM" = "cargobin" ]; then
    # Create as needed, but do not change a user override.
    create_config_symlink_as_needed "$WORKDIR_PARAM" "$HOME/.sui/sui_config"
  elif [ "$WORKDIR_PARAM" = "localnet" ]; then
    # User cannot override the localnet config, it is always config-default.
    set_config_symlink_force "$WORKDIR_PARAM" "$WORKDIRS/$WORKDIR_PARAM/config-default"
  else
    # Create as needed, but do not change a user override.
    create_config_symlink_as_needed "$WORKDIR_PARAM" "$WORKDIRS/$WORKDIR_PARAM/config-default"
  fi

  # Create the default suibase.yaml in case the user deleted it.
  #if [ ! -f "$WORKDIRS/$WORKDIR_PARAM/suibase.yaml" ]; then
  #  cp "$SCRIPTS_DIR/templates//$WORKDIR_PARAM/suibase.yaml" "$WORKDIRS/$WORKDIR_PARAM"
  #fi

  # Deprecated cd_sui_log_dir
}
export -f repair_workdir_as_needed

set_active_symlink_force() {
  WORKDIR_PARAM="$1"
  # Create or force the active symlink to the specified target.
  if [ ! -L "$WORKDIRS/active" ]; then
    ln -s "$WORKDIRS/$WORKDIR_PARAM" "$WORKDIRS/active"
  else
    update_ACTIVE_WORKDIR_var
    if [ "$ACTIVE_WORKDIR" != "$WORKDIR_PARAM" ]; then
      ln -nsf "$WORKDIRS/$WORKDIR_PARAM" "$WORKDIRS/active"
      update_ACTIVE_WORKDIR_var
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
  update_HOST_vars
  if [[ $HOST_PLATFORM == "Darwin" ]]; then
    # MacOS 'ps' works differently and does not show the $_ARGS to discern the
    # process, so next best thing is to match $_PROC to end-of-line with "$".
    #
    # Example of issue is to try to get_process_pid "sui" "start" and instead
    # getting the PID of "suibase-daemon".
    #
    # More info: https://github.com/ChainMovers/suibase/issues/79

    # shellcheck disable=SC2009
    _PID=$(ps x -o pid,comm | grep "$_PROC$" | grep -v -e grep | { head -n 1; cat >/dev/null 2>&1; } | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | { head -n 1; cat >/dev/null 2>&1; })
  else
    local _TARGET_CMD
    if [ -n "$ARGS" ]; then
      _TARGET_CMD="$_PROC $_ARGS"
    else
      _TARGET_CMD="$_PROC"
    fi

    # shellcheck disable=SC2009
    _PID=$(ps x -o pid,cmd 2>/dev/null | grep "$_TARGET_CMD" | grep -v grep | { head -n 1; cat >/dev/null 2>&1; } | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | { head -n 1; cat >/dev/null 2>&1; })
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
  _PID=$(get_process_pid "$SUI_BIN_DIR/sui" "start")
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

  # Deprecated cd_sui_log_dir

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
  update_SUI_PROCESS_PID_var
  if [ -n "$SUI_PROCESS_PID" ]; then
    echo "Stopping $WORKDIR (process pid $SUI_PROCESS_PID)"
    if $SUI_BASE_NET_MOCK; then
      unset SUI_PROCESS_PID
    else
      kill -s SIGTERM "$SUI_PROCESS_PID"
    fi

    # Make sure it is dead.
    end=$((SECONDS + 15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_SUI_PROCESS_PID_var
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

  exit_if_sui_binary_not_ok
  # Deprecated cd_sui_log_dir
  update_SUI_PROCESS_PID_var
  if [ -z "$SUI_PROCESS_PID" ]; then
    echo "Starting localnet process"
    sync_client_yaml no-proxy
    if $SUI_BASE_NET_MOCK; then
      SUI_PROCESS_PID=$SUI_BASE_NET_MOCK_PID
    else
      nohup env SUI_PROTOCOL_CONFIG_OVERRIDE_ENABLE=1 SUI_PROTOCOL_CONFIG_OVERRIDE_min_checkpoint_interval_ms=1000 RUST_LOG="error" "$SUI_BIN_DIR/sui" start --network.config "$NETWORK_CONFIG" >"$CONFIG_DATA_DIR/sui-process.log" 2>&1 &
    fi
    #NEW_PID=$!

    # Loop until "sui client" confirms being able to talk to the sui process, or exit
    # if that takes too long.
    end=$((SECONDS + 60))
    ((_mid_message = 30))
    ALIVE=false
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      if $SUI_BASE_NET_MOCK; then
        ALIVE=true
      else
        CHECK_ALIVE=$($SUI_BIN_ENV "$SUI_BIN_DIR/sui" client --client.config "$CLIENT_CONFIG" objects)
        # It is alive if first line either contains "Digest" or "No managed addresses". Both indicates
        # the sui process is running and responding to requests.
        if [[ "$CHECK_ALIVE" == *"igest"* ]] || [[ "$CHECK_ALIVE" == *"managed addresses"* ]]; then
          ALIVE=true
        fi
      fi
      if [ "$ALIVE" = true ]; then
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
        ((--_mid_message))
      fi
    done

    # Just UI aesthetic newline for when there was "." printed.
    if [ "$AT_LEAST_ONE_SECOND" = true ]; then
      echo
    fi

    # Act on success/failure of the sui process responding to "sui client".
    if [ "$ALIVE" = false ]; then
      echo "Sui process not responding. Try again? (may be the host is too slow?)."
      exit
    fi

    update_SUI_PROCESS_PID_var
    echo "localnet started (process pid $SUI_PROCESS_PID)"
    update_SUI_VERSION_var
    echo "$SUI_VERSION"

    # Apply the proper proxy settings (if any)
    sync_client_yaml
  fi
}
export -f start_sui_process

publish_clear_output() {
  local _DIR=$1
  # Only clear potential last publication.
  rm -f "$_DIR/publish-output.txt" >/dev/null 2>&1
  rm -f "$_DIR/publish-output.json" >/dev/null 2>&1
  rm -f "$_DIR/created-objects.json" >/dev/null 2>&1
  rm -f "$_DIR/package-id.json" >/dev/null 2>&1
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
export MOVE_TOML_PACKAGE_NAME
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
      true
      return
    fi
  else
    unset RESOLVED_SUI_REPO_DIR
  fi
  false
  return
}
export -f is_sui_repo_dir_override

is_sui_repo_dir_default() {
  # Just negate is_sui_repo_dir_override
  if is_sui_repo_dir_override; then
    false
    return
  fi
  true
  return
}
export -f is_sui_repo_dir_default

set_sui_repo_dir_default() {
  if is_sui_repo_dir_override; then
    rm -f "$SUI_REPO_DIR"
    echo "Removed set-sui-repo [$RESOLVED_SUI_REPO_DIR]"
  fi

  # Link to the default directory if already exists.
  if [ -d "$SUI_REPO_DIR_DEFAULT" ]; then
    set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT"
  else
    # No default directory.
    # Still a success as long the symlink is gone.
    echo "$WORKDIR using default sui repo"
  fi
}
export -f set_sui_repo_dir_default

set_sui_repo_dir() {

  local _OPTIONAL_PATH="$1"

  # User errors?
  if [ ! -d "$_OPTIONAL_PATH" ]; then
    setup_error "Path [ $_OPTIONAL_PATH ] not found"
  fi

  # The -n is important because target is a directory and without it
  # the command line arguments would be interpreted in the 3rd form
  # described in "man ln".
  ln -nsf "$_OPTIONAL_PATH" "$SUI_REPO_DIR"

  # Verify success.
  if is_sui_repo_dir_default; then
    echo "$WORKDIR using default sui repo [ $_OPTIONAL_PATH ]"
  else
    if is_sui_repo_dir_override; then
      echo "$WORKDIR set-sui-repo is now [ $_OPTIONAL_PATH ]"
    else
      setup_error "$WORKDIR set-sui-repo failed [ $_OPTIONAL_PATH ]"
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
  _SUI_ERR=$("$SUI_EXEC" client gas "$1" --json 2>&1 | grep -iE "error|invalid|help")
  if [ -n "$_SUI_ERR" ]; then
    error_exit "Invalid hexadecimal Sui address [$1]."
  fi
}
export -f exit_if_not_valid_sui_address

export ACTIVE_ADDRESS=""
update_ACTIVE_ADDRESS_var() {
  local _SUI_BINARY="$1"
  local _CLIENT_FILE="$2"
  ACTIVE_ADDRESS=""
  # Get the active address by querying the client.
  local _ADDR
  local _CMD_OUTPUT
  local _RETRY_COUNT=0
  local _MAX_RETRIES=2

  while [ $_RETRY_COUNT -lt $_MAX_RETRIES ]; do
    _CMD_OUTPUT=$($SUI_BIN_ENV "$_SUI_BINARY" client --client.config "$_CLIENT_FILE" active-address 2>/dev/null)
    _ADDR=$(echo "$_CMD_OUTPUT" | grep "0x")
    if [[ "$_ADDR" =~ 0x[[:xdigit:]]+ ]]; then
      _ADDR="${BASH_REMATCH[0]}"
    fi
    # TODO Better validation that the address is valid!?
    if [ -n "$_ADDR" ]; then
      ACTIVE_ADDRESS="$_ADDR"
      break
    else
      if [ $_RETRY_COUNT -eq 0 ]; then
        echo "Getting active address for ${WORKDIR_NAME}"
      fi
      wait_for_json_rpc_up "${WORKDIR_NAME}"
      _RETRY_COUNT=$((_RETRY_COUNT + 1))
    fi
  done

  if [ -z "$ACTIVE_ADDRESS" ]; then
      warn_user "Unable to get active address from [$_CLIENT_FILE]"
  fi
}
export -f update_ACTIVE_ADDRESS_var

clear_active_address_field() {
  local _CLIENT_FILE="$1"
  # Unset the active_address, to force the default.
  if [ -f "$_CLIENT_FILE" ]; then
    sed -i.bak -e 's/active_address:.*/active_address: ~/' "$_CLIENT_FILE" && rm "$_CLIENT_FILE.bak"
  fi
}
export -f clear_active_address_field

add_test_addresses() {

  if [[ "${CFG_auto_key_generation:?}" == 'false' ]]; then
    return
  fi

  # Add 15 addresses to the specified client.yaml (sui.keystore)
  # The _WORDS_FILE is what is normally appended in "recovery.txt"
  local _SUI_BINARY=$1
  local _CLIENT_FILE=$2
  local _WORDS_FILE=$3

  wait_for_json_rpc_up "${WORKDIR_NAME}"

  # Track the highest ED25519 address during creation. This will tentatively become
  # the active address later.
  local _HIGH_ADDR=""
  local _OUTPUT
  local _ADDRESS

  {
    echo "["
    for _i in {1..5}; do
      _OUTPUT=$($SUI_BIN_ENV "$_SUI_BINARY" client --client.config "$_CLIENT_FILE" new-address ed25519 --json 2>/dev/null)
      echo "$_OUTPUT,"

      # Extract address from output and compare with highest so far
      if [[ "$_OUTPUT" =~ \"address\":\ *\"(0x[0-9a-fA-F]+)\" ]]; then
        _ADDRESS="${BASH_REMATCH[1]}"
        if [[ -z "$_HIGH_ADDR" || "$_ADDRESS" > "$_HIGH_ADDR" ]]; then
          _HIGH_ADDR="$_ADDRESS"
        fi
      fi

      $SUI_BIN_ENV "$_SUI_BINARY" client --client.config "$_CLIENT_FILE" new-address secp256k1 --json 2>/dev/null
      echo ","
      $SUI_BIN_ENV "$_SUI_BINARY" client --client.config "$_CLIENT_FILE" new-address secp256r1 --json 2>/dev/null
      if [ "$_i" -lt 5 ]; then
        echo ","
      fi
    done
    echo "]"
  } >>"$_WORDS_FILE"

  # Logic to select the default test address.
  #
  # The _WORDS_FILE look somthing like this:
  #
  # [{
  #  "alias": "agitated-zircon",
  #  "address": "0x09bc4b4a704bf8994ba28b3dfcaee530540963153fdfe1c8ab897b459197d73d",
  #  "keyScheme": "ED25519",
  #  "recoveryPhrase": "impulse crater hockey gun fine remember member depend shine episode venture damp"
  #  }
  #  ,
  #  {
  #    "alias": "blissful-avanturine",
  #    "address": "0x7f23e100ea0da802a683cd7261f6ddadf8bbc2968b0c1c7834361366ee09b452",
  #    "keyScheme": "ED25519",
  #    "recoveryPhrase": "lend banner script damage afford express sunny tiger useful sustain tuition cart"
  #  }
  #  ,
  #  {
  #    "alias": "romantic-felspar",
  #    "address": "0xef7ee5a26cc80426d22e0470c051063c6631055f19fea5cd53fab129f9dbccf4",
  #    "keyScheme": "Secp256r1",
  #    "recoveryPhrase": "pool poet dignity ceiling alcohol glove interest idle poet sure balcony stock"
  #  }
  # ]
  #


  # Set highest address as active. Best-effort... just warn if fails.
  local _SET_ACTIVE_SUCCESS=false
  if [[ "$_HIGH_ADDR" =~ 0x[[:xdigit:]]+ ]]; then
    _HIGH_ADDR="${BASH_REMATCH[0]}"

    $SUI_BIN_ENV "$_SUI_BINARY" client --client.config "$_CLIENT_FILE" switch --address "$_HIGH_ADDR" >/dev/null 2>&1

    # Test that it succeeded.
    update_ACTIVE_ADDRESS_var "$_SUI_BINARY" "$_CLIENT_FILE"
    if [ -n "$ACTIVE_ADDRESS" ]; then
      if [ "$ACTIVE_ADDRESS" == "$_HIGH_ADDR" ]; then
        _SET_ACTIVE_SUCCESS=true
      else
        echo "Getting [$ACTIVE_ADDRESS] instead of [$_HIGH_ADDR]"
        warn_user "Unable to confirm active address set (1)."
      fi
    else
      echo "Trying to set active address to [$_HIGH_ADDR]"
      warn_user "Unable to confirm active address set (2)."
    fi
  else
    warn_user "Unable to find highest address in [$_CLIENT_FILE]"
  fi

  if [ "$_SET_ACTIVE_SUCCESS" = false ]; then
    # Unset the active_address, to force the default.
    clear_active_address_field "$_CLIENT_FILE"
  fi
}
export -f add_test_addresses

is_base16() {
  local _INPUT="$1"
  # Return true if is a valid hexadecimal string.
  # Ignore leading "0x" if any
  _INPUT="${_INPUT#0x}"
  if [[ "$_INPUT" =~ ^[0-9a-fA-F]+$ ]]; then
    true
    return
  fi
  false
}
export -f is_base16

check_is_valid_base64_keypair() {
  # Return true if the parameter is a valid base64 keypair format
  # (as stored in a sui.keystore).
  local _KEYPAIR="$1"

  # Convert from Base64 to hexadecimal string.
  _bytes=$(echo -n "$_KEYPAIR" | base64 -d 2>&1 | xxd -p -c33 | tr -d '[:space:]')
  # Check that the string does not have the word "Invalid" or "Error" in it...
  if [[ "${_bytes}" != *"nvalid"* && "${_bytes}" != *"rror"* ]]; then
    # Check that the string is exactly 33 bytes (66 characters)
    if [ ${#_bytes} -eq 66 ]; then
      true
      return
    fi
  fi
  false
  return
}
export -f check_is_valid_base64_keypair

check_is_valid_base16_keypair() {
  # Return true if the parameter is a valid base64 keypair format
  # (as stored in a sui.keystore).
  local _KEYPAIR="$1"

  # Convert from Base64 to hexadecimal string.
  _bytes=$(echo -n "$_KEYPAIR" | xxd -p -c33 | tr -d '[:space:]')
  # Check that the string does not have the word "Invalid" or "Error" in it...
  if [[ "${_bytes}" != *"nvalid"* && "${_bytes}" != *"rror"* ]]; then
    # Check that the string is exactly 33 bytes (66 characters)
    if [ ${#_bytes} -eq 66 ]; then
      true
      return
    fi
  fi
  false
  return
}
export -f check_is_valid_base64_keypair

check_is_valid_hex_pk() {
  # Return true if the parameter is a valid hexadecimal public key.
  # Length must be 64 characters (without 0x).
  # If 0x is specified, just ignore it.
  # Verify that the string is a valid hexadecimal string.
  local _PK="$1"
  if [ -z "$_PK" ]; then
    false
    return
  fi
  if [[ "$_PK" == 0x* ]]; then
    _PK="${_PK#0x}"
  fi
  if [ ${#_PK} -eq 64 ] && is_base16 "$_PK"; then
    true
    return
  fi
  false
  return
}
export -f check_is_valid_hex_pk

export ACTIVE_KEYSTORE=()
load_ACTIVE_KEYSTORE() {
  # Set second parameter to true to enforce additional validation (slower).
  # If not set, then just do a quick check.
  local _SRC="$1"
  local _SLOW_VALIDATION="$2"

  ACTIVE_KEYSTORE=()

  if [ ! -f "$_SRC" ]; then
    return
  fi

  # If second parameter is not set, then set it to false.
  if [ -z "$_SLOW_VALIDATION" ]; then
    _SLOW_VALIDATION=false
  fi

  # Load the sui.keystore elements into ACTIVE_KEYSTORE (a bash array).
  #
  # The _SRC is the path to a file. It contains a JSON array of strings.
  #
  # Example of content:
  #  [
  #   "AOToawZbfMNATU6KPldYuoGQpp82BE0w5BknPCTBjgXT",
  #   "AAYp6dagpe5U055xhXEFeAfvpg5CL37tJLbWd2TwsgIF",
  #   "APDKm1PElnKl8ho8uNhpM552kdJznTT+bg1UZCjANF+V",
  #   "AmTXXoiEVTdpy3pBWVAaAWx5baNanBN21NshmAiSPDWW",
  #  ]
  #
  # Load the file into a bash array.
  local _keyvalue
  while IFS= read -r _keyvalue; do
    # Validate the line is a valid input.
    # Trim space at start/end of line.
    _keyvalue=$(echo "$_keyvalue" | xargs)
    # Skip empty lines.
    if [ -z "$_keyvalue" ]; then
      continue
    fi
    if [ "$_SLOW_VALIDATION" = true ]; then
      if ! check_is_valid_base64_keypair "$_keyvalue"; then
        error_exit "Invalid keypair [$_keyvalue] in $_SRC"
      fi
    else
      # Just do a sanity test that the string is at least 41 characters long and not
      # starting with "0x".
      if [ "${#_keyvalue}" -lt 41 ] || [[ "$_keyvalue" == 0x* ]]; then
        error_exit "Invalid keypair [$_keyvalue] in $_SRC"
      fi
    fi
    ACTIVE_KEYSTORE+=("$_keyvalue")
  done < <(grep -Eo '"[^"]*"|[^,]*' "$_SRC" | tr -d '[]"')
}
export -f load_ACTIVE_KEYSTORE

write_ACTIVE_KEYSTORE() {
  local _DST_FINAL="$1"

  # Write the ACTIVE_KEYSTORE array to _DST file.
  # JSON Format is:
  # [
  #    <element1>,
  #    <element2>,
  # ]

  # Work with a temp file and make the write
  # to the final file atomic with a "mv -f".
  local _DST
  _DST=$(mktemp)
  echo -n "[" >|"$_DST"
  local _firstline=true
  local _keyvalue
  for _keyvalue in "${ACTIVE_KEYSTORE[@]}"; do
    if [ "$_firstline" = true ]; then
      _firstline=false
    else
      echo -n "," >>"$_DST"
    fi
    echo >>"$_DST"
    echo -n "  \"$_keyvalue\"" >>"$_DST"
  done
  echo >>"$_DST"
  echo -n "]" >>"$_DST"
  mv -f "$_DST" "$_DST_FINAL"
}
export -f write_ACTIVE_KEYSTORE

create_empty_keystore_file() {
  # Call with care... will overwrite the existing keystore without blinking.
  if [ "$#" -ne 1 ]; then
    error_exit "create_empty_keystore_file() requires 1 parameter"
  fi
  if [ -z "$1" ]; then
    error_exit "create_empty_keystore_file() requires a non-empty parameter"
  fi
  local _DIR=$1
  local _DST_FILE="$_DIR/sui.keystore"
  # Wipe out the keystore.
  mkdir -p "$_DIR"
  rm -rf "$_DST_FILE" >/dev/null 2>&1
  printf '[\n]' >|"$_DST_FILE"
}

array_contains() {
  # Check for coding errors.
  if [ "$#" -ne 2 ]; then
    error_exit "array_contains() requires 2 parameters"
  fi

  # Return true if the array (first parameter) contains the element (second parameter)
  local _ARRAY="$1[@]"
  local _ELEMENT="$2"
  local _e

  if [ -z "$_ELEMENT" ]; then
    # Array can't contain empty string, so can't be in the array.
    false
    return
  fi

  # Check if the array is empty.
  if [ ${#_ARRAY} -eq 0 ]; then
    false
    return
  fi

  # Linear search in the array.
  for _e in "${!_ARRAY}"; do
    if [[ "$_e" == "$_ELEMENT" ]]; then
      true
      return
    fi
  done
  false
  return
}
export -f array_contains

copy_private_keys_yaml_to_keystore() {
  if [ "$WORKDIR" = "cargobin" ]; then
    return
  fi

  # Load private keys from suibase.yaml into a sui.keystore
  # Do not duplicate key.
  local _DST="$1"

  # Do nothing if variable CFG_add_private_keys_ is not set
  if [ -z "${CFG_add_private_keys_:-}" ]; then
    return
  fi

  # Do a fast load (assume the content is valid).
  load_ACTIVE_KEYSTORE "$_DST" false

  # Example of suibase.yaml:
  #
  # add_private_keys:
  #  - 0x937273cdae34592736ab25dcad423a4adfae3a4d
  #  - AIFdx03sdsjEDFSSMakjdhyRuejiS

  #
  # Do a first pass to parse/convert as much as possible from
  # the suibase.yaml.
  #
  # Will put in an array all the private keys to be tentatively
  # added. Duplicate entries are silently dropped.
  #
  # If they are all good, then the new keys are imported to the
  # destination sui.keystore in a second pass.
  #
  # The second pass is slower, but more strict on validation.
  local _KEYS_TO_ADD=()
  for _keyvar in ${CFG_add_private_keys_:?}; do
    local _original_keyvalue
    local _keyvalue
    local _word_count
    _original_keyvalue=${!_keyvar}
    _keyvalue=$_original_keyvalue
    _wordcount="$(echo "$_keyvalue" | wc -w)"

    # if more than 5 words on the line, assume it is an attempt at inserting a mnemonic string.
    if [ "$_wordcount" -gt 5 ]; then
      # Look for an exact count of 24 words.
      if [ ! "$_wordcount" -eq 24 ]; then
        error_exit "add_private_keys mnemonic should be exactly 24 words [$_keyvalue]"
      fi
    else
      # Check if _keyvalue string has unexpectably more than one word.
      if [ "$_wordcount" -gt 1 ]; then
        error_exit "add_private_keys should not have more than one value per line in suibase.yaml [$_keyvalue]"
      fi

      # If start with 0x, then try first with converting directly with keytool.
      if [[ "${_keyvalue}" == "0x"* ]]; then
        _cnvt_attempt=$($NOLOG_KEYTOOL_BIN --json convert "$_keyvalue" 2>&1)
        # Check if --json supported, if not, try with older version of keytool.
        if [[ $_cnvt_attempt == *"json"* && $_cnvt_attempt == *"rror"* ]]; then
          # Keep only the last line which should be the key pair translated to Base64.
          # Send to /dev/null the *successful* messages on stderr!
          _keyvalue=$($NOLOG_KEYTOOL_BIN convert "$_keyvalue" 2>/dev/null | tail -n 1)
        else
          # Extract key from JSON
          _keyvalue=$_cnvt_attempt
          update_JSON_VALUE "base64WithFlag" "$_keyvalue"
          if [ -z "$_keyvalue" ]; then
            update_JSON_VALUE "hexWithoutFlag" "$_keyvalue"
            if [ -z "$_keyvalue" ]; then
              error_exit "could not extract key from json [$_cnvt_attempt]"
            fi
          fi
          _keyvalue=$JSON_VALUE
        fi

        # Some version of the Sui client returns in hex with missing leading
        # zeroes... (but the client help shows base64!). Just compensate for
        # the inconsistency here and fix things up with a few assumptions.
        if is_base16 "$_keyvalue"; then
          # Remove leading 0x if somehow any.
          _keyvalue=${_keyvalue#0x}
          # Add missing leading zeroes if less than 66 characters.
          while [ ${#_keyvalue} -lt 66 ]; do
            _keyvalue="0${_keyvalue}"
          done
          # Convert _keyvalue to base64.
          _keyvalue=$(echo "$_keyvalue" | xxd -r -p | base64)
        fi
      fi

      # If the $_keyvalue is already in the sui.keystore, assume it is
      # valid and skip it.
      if array_contains ACTIVE_KEYSTORE "$_keyvalue"; then
        continue
      fi

      if check_is_valid_base64_keypair "$_keyvalue"; then
        _KEYS_TO_ADD+=("$_keyvalue")
        continue
      fi

      error_exit Invalid private key format ["$_original_keyvalue"]
    fi
  done

  # Nothing more to do if there is no key to add.
  if [ "${#_KEYS_TO_ADD[@]}" -eq 0 ]; then
    return
  fi

  # Now do a slow load to make sure everything is valid.
  load_ACTIVE_KEYSTORE "$_DST" true

  # Merge _KEYS_TO_ADD into ACTIVE_KEYSTORE. Remove duplicates.
  local _NEW_KEYSTORE=()
  for _keyvalue in "${ACTIVE_KEYSTORE[@]}" "${_KEYS_TO_ADD[@]}"; do
    if ! array_contains _NEW_KEYSTORE "$_keyvalue"; then
      _NEW_KEYSTORE+=("$_keyvalue")
    fi
  done

  # Write the end result (if something changed).
  if [ "${#ACTIVE_KEYSTORE[@]}" -ne "${#_NEW_KEYSTORE[@]}" ]; then
    ACTIVE_KEYSTORE=("${_NEW_KEYSTORE[@]}")
    echo "Updating $_DST"
    write_ACTIVE_KEYSTORE "$_DST"
  fi
}
export -f copy_private_keys_yaml_to_keystore

update_client_yaml_active_address() {
  # Update the client.yaml active address field if not set.
  # (a client call switch to an address, using output of another client call picking a default).
  STR_FOUND=$(grep "active_address:" "$CLIENT_CONFIG" | grep "~")
  if [ -n "$STR_FOUND" ]; then
    update_ACTIVE_ADDRESS_var "$SUI_BIN_DIR/sui" "$CLIENT_CONFIG"
    if [ -n "$ACTIVE_ADDRESS" ]; then
      $SUI_BIN_ENV "$SUI_BIN_DIR"/sui client --client.config "$CLIENT_CONFIG" switch --address "$ACTIVE_ADDRESS"
    fi
  fi
}
export -f update_client_yaml_active_address

# Adaptation of https://stackoverflow.com/questions/1955505/parsing-json-with-unix-tools
#
# Can extract only a simple "key":"value" pair embedded anywhere within the json.
#
# Update global JSON_VALUE to return value... a bit hacky but works surprisingly well.
#
export JSON_VALUE=""
update_JSON_VALUE() {
  local key=$1
  local json=$2

  local string_regex='"([^"\]|\\.)*"'
  local number_regex='-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?'
  local value_regex="${string_regex}|${number_regex}|true|false|null"
  local pair_regex="\"${key}\"[[:space:]]*:[[:space:]]*(${value_regex})"

  if [[ ${json} =~ ${pair_regex} ]]; then
    # Get the value from the regex.
    JSON_VALUE="${BASH_REMATCH[1]}"
    # Replace the escaped \" with single quote.
    JSON_VALUE=${JSON_VALUE//\\\"/\'}
    # Remove the surrounding double-quote
    JSON_VALUE="${JSON_VALUE//\"/}"
  else
    JSON_VALUE=""
  fi
}
export -f update_JSON_VALUE

sync_client_yaml() {

  # unset for normal call
  #
  # "no-proxy" when proxy must not be used.
  local _CMD="$1"

  # Generally synchronize client.yaml using the suibase.yaml proxy settings.
  #
  # This check and potentially switch the client.yaml 'env' depending
  # of the proxy being enabled or not.
  #
  local _TARGET_YAML="$WORKDIRS/$WORKDIR_NAME/config/client.yaml"
  if [ -z "$WORKDIR_NAME" ] || [ ! -f "$_TARGET_YAML" ]; then
    return
  fi

  local _ACTIVE_ENV
  _ACTIVE_ENV=$(grep active_env "$_TARGET_YAML" | tr -d '[:space:]')
  _ACTIVE_ENV=${_ACTIVE_ENV#*:} # Remove the "active_env:" prefix

  local _EXPECTED_ENV
  _EXPECTED_ENV=$WORKDIR_NAME
  if [ "$_CMD" != "no-proxy" ] && [ "${CFG_proxy_enabled:?}" != "false" ]; then
    local _USER_REQUEST
    _USER_REQUEST=$(get_key_value "$WORKDIR_NAME" "user_request")
    if [ "$_USER_REQUEST" != "stop" ]; then
      # Proxy is enabled and workdir is running, so
      # the client env should be toward the proxy.
      _EXPECTED_ENV=$_EXPECTED_ENV"_proxy"
    fi
  fi

  if [ "$_ACTIVE_ENV" != "$_EXPECTED_ENV" ]; then
    echo "Switching sui client env from [$_ACTIVE_ENV] to [$_EXPECTED_ENV]"
    $SUI_EXEC client switch --env "$_EXPECTED_ENV" >/dev/null 2>&1
    # Verify if successful. If not and the _EXPECTED_ENV is for the
    # proxy, then try to "fix" the client.yaml.
    _ACTIVE_ENV=$(grep active_env "$_TARGET_YAML" | tr -d '[:space:]')
    _ACTIVE_ENV=${_ACTIVE_ENV#*:} # Remove the "active_env:" prefix
    if [ "$_ACTIVE_ENV" != "$_EXPECTED_ENV" ]; then
      # Repair if the _proxy" is missing in client.yaml
      local _PROXY_IN
      _PROXY_IN=$(grep "$_EXPECTED_ENV" "$_TARGET_YAML" | tr -d '[:space:]')
      if [ -z "$_PROXY_IN" ]; then
        # Note: it is important to escape the first two space for the sed /a command to work.
        _NEW_ENV="envs:\n  - alias: $_EXPECTED_ENV\n    rpc: \"http://${CFG_proxy_host_ip:?}:${CFG_proxy_port_number:?}\"\n    ws: ~"
        # Insert the new links after the line starting with "envs:" in client.yaml
        sed -i.bak "s+^envs:+$_NEW_ENV+g" "$_TARGET_YAML" && rm "$_TARGET_YAML.bak"
        echo "[$_EXPECTED_ENV] added to client.yaml"
      fi
      # Try again.
      $SUI_EXEC client switch --env "$_EXPECTED_ENV" >/dev/null 2>&1
      _ACTIVE_ENV=$(grep active_env "$_TARGET_YAML" | tr -d '[:space:]')
      _ACTIVE_ENV=${_ACTIVE_ENV#*:} # Remove the "active_env:" prefix
      if [ "$_ACTIVE_ENV" != "$_EXPECTED_ENV" ]; then
        warn_user "Failed to switch sui client env to [$_EXPECTED_ENV]."
      fi
    fi
    if [ "$_EXPECTED_ENV" == "${WORKDIR_NAME}_proxy" ]; then
      # Block until verified that the proxy is responding (or timeout).
      wait_for_json_rpc_up "${WORKDIR_NAME}"
    fi
  fi
}
export -f sync_client_yaml

# Functions to get the "pre-compiled binary" release assets information from github.
#
# PRECOMP_REMOTE is a boolean indicating if the repo has a binary.
#
# When true other PRECOMP_REMOTE_XXXXXX variables are set with related
# information.
#
# Exit on errors.
#
# For now the only supported repo is github.
export PRECOMP_REMOTE=""
export PRECOMP_REMOTE_PLATFORM=""
export PRECOMP_REMOTE_ARCH=""
export PRECOMP_REMOTE_VERSION=""
export PRECOMP_REMOTE_TAG_NAME=""
export PRECOMP_REMOTE_DOWNLOAD_URL=""
export PRECOMP_REMOTE_DOWNLOAD_DIR=""
export PRECOMP_REMOTE_FILE_NAME_VERSION=""

update_PRECOMP_REMOTE_var() {
  PRECOMP_REMOTE="false"
  PRECOMP_REMOTE_PLATFORM=""
  PRECOMP_REMOTE_ARCH=""
  PRECOMP_REMOTE_VERSION=""
  PRECOMP_REMOTE_TAG_NAME=""
  PRECOMP_REMOTE_DOWNLOAD_URL=""
  PRECOMP_REMOTE_DOWNLOAD_DIR=""
  PRECOMP_REMOTE_FILE_NAME_VERSION=""

  local _REPO_URL="${CFG_default_repo_url:?}"
  local _BRANCH="${CFG_default_repo_branch:?}"

  # Make sure _REPO is github (start with "https://github.com")
  if [[ "$_REPO_URL" != "https://github.com"* ]]; then
    setup_error "repo [$_REPO_URL] not supported for pre-compiled binaries"
  fi

  # Change the URL to the API URL (prepend 'api.' before github.com and '/repos' after)
  _REPO_URL="${_REPO_URL/github.com/api.github.com/repos}"

  # Remove the trailing .git in the URL
  # _REPO_URL is now the URL prefix for all github API call.
  _REPO_URL="${_REPO_URL%.git}"

  # Identify the platform and arch substrings in the asset to download.
  local _BIN_PLATFORM # "ubuntu", "macos" or "windows".
  local _BIN_ARCH     # "arm64" or "x86_64"
  update_HOST_vars
  if [ "$HOST_PLATFORM" = "Linux" ]; then
    _BIN_PLATFORM="ubuntu"
    _BIN_ARCH="$HOST_ARCH"
  else
    if [ "$HOST_PLATFORM" = "Darwin" ]; then
      _BIN_PLATFORM="macos"
      _BIN_ARCH="$HOST_ARCH"
    else
      setup_error "Unsupported platform [$HOST_PLATFORM] and arch [$HOST_ARCH]"
    fi
  fi

  local _OUT
  local _TAG_NAME
  local _FORCE_TAG_NAME
  local _DOWNLOAD_URL
  local _DOWNLOAD_SUBSTRING="$_BIN_PLATFORM-$_BIN_ARCH"

  if [ "${CFG_force_tag:?}" != "~" ]; then
    _FORCE_TAG_NAME="${CFG_force_tag:?}"
    echo "suibase.yaml: Forcing to use tag '[$_FORCE_TAG_NAME]'"
  fi

  update_USER_GITHUB_TOKEN_var

  for _retry_curl in 1 2 3; do
    _DOWNLOAD_URL=""
    _TAG_NAME=""
    if [ -n "$USER_GITHUB_TOKEN" ]; then
      _OUT=$(curl -s --request GET \
        --url "$_REPO_URL/releases" \
        --header "X-GitHub-Api-Version: 2022-11-28" \
        --header "Authorization: Bearer $USER_GITHUB_TOKEN")
    else
      _OUT=$(curl -s --request GET \
        --url "$_REPO_URL/releases" \
        --header "X-GitHub-Api-Version: 2022-11-28")
    fi

    if [ -z "$_OUT" ]; then
      if [ $_retry_curl -lt 3 ]; then
        continue
      else
        setup_error "Failed to get release information from [$_REPO_URL]"
      fi
    fi

    while read -r line; do
      # Return something like: "tag_name": "testnet-v1.8.2",
      _TAG_NAME="${line#*\:}"      # Remove the ":" and everything before
      _TAG_NAME="${_TAG_NAME#*\"}" # Remove the first '"' and everything before
      _TAG_NAME="${_TAG_NAME%\"*}" # Remove the last '"' and everything after

      local _DISPLAY_FOUND
      _DISPLAY_FOUND=false
      if [ "$DEBUG_PARAM" = "true" ]; then
        _DISPLAY_FOUND=true
      fi

      if [ -n "$_FORCE_TAG_NAME" ]; then
        if [ $_retry_curl -lt 2 ]; then
          _DISPLAY_FOUND=true
        fi
      fi

      if [ "$_DISPLAY_FOUND" = "true" ]; then
        echo "Found $_TAG_NAME in remote repo"
      fi

      # Find the binary asset for that release.
      _DOWNLOAD_URL=$(echo "$_OUT" | grep "browser_download_url" | grep "$_DOWNLOAD_SUBSTRING" | grep "$_TAG_NAME" | sort -r | { head -n 1; cat >/dev/null 2>&1; })
      _DOWNLOAD_URL="${_DOWNLOAD_URL#*\:}" # Remove the ":" and everything before
      _DOWNLOAD_URL="${_DOWNLOAD_URL#*\"}" # Remove the first '"' and everything before
      _DOWNLOAD_URL="${_DOWNLOAD_URL%\"*}" # Remove the last '"' and everything after

      # Stop looping if _DOWNLOAD_URL looks valid.
      if [ -n "$_DOWNLOAD_URL" ]; then
        if [ -n "$_FORCE_TAG_NAME" ]; then
          if [ "$_TAG_NAME" == "$_FORCE_TAG_NAME" ]; then
            break
          fi
        elif is_valid_assets "$_TAG_NAME" "$_BIN_PLATFORM" "$_BIN_ARCH"; then
          break
        else
          echo "Warn: Skipping invalid Mysten Labs assets $_TAG_NAME"
        fi
      fi
    done <<<"$(echo "$_OUT" | grep "tag_name" | grep "$_BRANCH" | sort -rV)"

    # Stop looping for retry if _DOWNLOAD_URL looks valid.
    # TODO Refactor this to avoid duplicate logic done in above loop.
    if [ -n "$_DOWNLOAD_URL" ]; then
      if [ -n "$_FORCE_TAG_NAME" ]; then
        if [ "$_TAG_NAME" == "$_FORCE_TAG_NAME" ]; then
          break
        fi
      elif is_valid_assets "$_TAG_NAME" "$_BIN_PLATFORM" "$_BIN_ARCH"; then
        break
      fi
    fi

    # Something went wrong.
    if [ "$DEBUG_PARAM" = "true" ]; then
      echo "Github API call result = [$_OUT]"
    fi

    if [ -n "${USER_GITHUB_TOKEN}" ] && [[ "$_OUT" == *"Bad credentials"* ]]; then
      setup_error "The github_token [${USER_GITHUB_TOKEN}] in suibase.yaml seems invalid."
    fi

    if [[ "$_OUT" == *"rate limit exceeded"* ]]; then
      if [ -z "${USER_GITHUB_TOKEN}" ]; then
        warn_user "Consider adding your github_token in suibase.yaml to increase rate limit."
      fi
      setup_error "Github rate limit exceeded. Please try again later."
    fi

    if [ $_retry_curl -lt 2 ]; then
      warn_user "Could not retreive release information. Retrying"
    fi
    _DOWNLOAD_URL=""
  done # curl retry loop

  if [ -z "$_DOWNLOAD_URL" ]; then
    if [ -n "$_FORCE_TAG_NAME" ]; then
      if [ "$_TAG_NAME" != "$_FORCE_TAG_NAME" ]; then
        echo "suibase.yaml: tag [$_FORCE_TAG_NAME] not found in remote repo"
        setup_error "Verify force_tag in suibase.yaml is a valid tag for [$_REPO_URL]"
      fi
    else
      setup_error "Could not find a '$_DOWNLOAD_SUBSTRING' binary asset for $_BRANCH in [$_REPO_URL]"
    fi
  fi

  local _TAG_VERSION="${_TAG_NAME#*\-v}" # Remove '-v' and everything before.
  # echo "_OUT=$_OUT"
  # echo "_TAG_NAME=$_TAG_NAME"
  # echo "_TAG_VERSION=$_TAG_VERSION"
  # echo _DOWNLOAD_URL="$_DOWNLOAD_URL"

  # All good. Return success.
  PRECOMP_REMOTE="true"
  PRECOMP_REMOTE_PLATFORM="$_BIN_PLATFORM"
  PRECOMP_REMOTE_ARCH="$_BIN_ARCH"
  PRECOMP_REMOTE_VERSION="$_TAG_VERSION"
  PRECOMP_REMOTE_TAG_NAME="$_TAG_NAME"
  PRECOMP_REMOTE_DOWNLOAD_URL="$_DOWNLOAD_URL"

  return
}
export -f update_PRECOMP_REMOTE_var

is_valid_assets() {
  local _TAG_NAME="$1" # "devnet-v1.25.0"
  local _PLATFORM="$2" # "ubuntu", "macos"
  local _ARCH="$3"     # "arm64", "x86_64"

  local _IS_VALID=true

  # Add here detection of tags known to be defective ("bad release").

  # macos arm64 v1.25 now fix:
  #    https://github.com/MystenLabs/sui/issues/17770
  #
  # if [[ "$_PLATFORM" == "macos" ]] && [[ "$_ARCH" == "arm64" ]]; then
  #  if [[ "$_TAG_NAME" == *"v1.25.0"* ]]; then
  #    _IS_VALID=false
  #  fi
  # fi

  if $_IS_VALID; then
    true
    return
  else
    false
    return
  fi
}
export -f is_valid_assets

download_PRECOMP_REMOTE() {
  local _WORKDIR="$1"
  PRECOMP_REMOTE_DOWNLOAD_DIR=""
  PRECOMP_REMOTE_FILE_NAME_VERSION=""

  # It is assumed update_PRECOMP_REMOTE_var() was successfully called before
  # and there is indeed something to download and install.
  if [ "$PRECOMP_REMOTE" != "true" ]; then
    return
  fi

  # Download the $PRECOMP_REMOTE_DOWNLOAD_URL into .cache/precompiled_downloads/<branch name>
  local _BRANCH="${CFG_default_repo_branch:?}"
  local _DOWNLOAD_DIR="$WORKDIRS/$_WORKDIR/.cache/precompiled_downloads/$_BRANCH"
  mkdir -p "$_DOWNLOAD_DIR"
  local _DOWNLOAD_FILENAME="${PRECOMP_REMOTE_DOWNLOAD_URL##*/}"
  local _DOWNLOAD_FILENAME_WITHOUT_TGZ="${_DOWNLOAD_FILENAME%.tgz}"
  local _DOWNLOAD_FILEPATH="$_DOWNLOAD_DIR/$_DOWNLOAD_FILENAME"
  local _EXTRACT_DIR="$_DOWNLOAD_DIR/$_DOWNLOAD_FILENAME_WITHOUT_TGZ" # Where the .tgz content will be placed.

  local _USE_VERSION=""

  # Location prior to release 1.23 from Mysten Labs.
  local _EXTRACTED_DIR_V1="$_EXTRACT_DIR/target/release"
  local _EXTRACTED_TEST_FILENAME_V1="sui-$PRECOMP_REMOTE_PLATFORM-$PRECOMP_REMOTE_ARCH"
  local _EXTRACTED_TEST_FILEDIR_V1="$_EXTRACTED_DIR_V1/$_EXTRACTED_TEST_FILENAME_V1"

  # Location starting with release 1.23 from Mysten Labs.
  local _EXTRACTED_DIR_V2="$_EXTRACT_DIR"
  local _EXTRACTED_TEST_FILENAME_V2="sui"
  local _EXTRACTED_TEST_FILEDIR_V2="$_EXTRACTED_DIR_V2/$_EXTRACTED_TEST_FILENAME_V2"

  # These will be initialized with the version detected in the downloaded file.
  local _EXTRACTED_DIR
  local _EXTRACTED_TEST_FILEDIR

  # TODO validate here the local file is really matching the remote in case of republishing?

  # Try twice before giving up.
  for i in 1 2; do
    # Download if not already done.
    local _DO_EXTRACTION="false"
    if [ -f "$_DOWNLOAD_FILEPATH" ]; then
      if [ ! -f "$_EXTRACTED_TEST_FILEDIR_V1" ] && [ ! -f "$_EXTRACTED_TEST_FILEDIR_V2" ]; then
        _DO_EXTRACTION="true"
      fi
    else
      echo "Downloading precompiled $_DOWNLOAD_FILENAME"
      update_USER_GITHUB_TOKEN_var
      if [ -n "$USER_GITHUB_TOKEN" ]; then
        echo "Using github_token"
        curl -s -L -o "$_DOWNLOAD_FILEPATH" "$PRECOMP_REMOTE_DOWNLOAD_URL" \
          --header "X-GitHub-Api-Version: 2022-11-28" \
          --header "Authorization: Bearer $USER_GITHUB_TOKEN"
      else
        curl -s -L -o "$_DOWNLOAD_FILEPATH" "$PRECOMP_REMOTE_DOWNLOAD_URL" \
          --header "X-GitHub-Api-Version: 2022-11-28"
      fi

      # Extract if not already done. This is an indirect validation that the downloaded file is OK.
      # If not OK, delete and try download again.
      _DO_EXTRACTION="true"
    fi

    if [ "$_DO_EXTRACTION" = "true" ]; then
      # echo "Extracting into $_EXTRACT_DIR"
      rm -rf "$_EXTRACT_DIR" >/dev/null 2>&1
      mkdir -p "$_EXTRACT_DIR"
      tar -xzf "$_DOWNLOAD_FILEPATH" -C "$_EXTRACT_DIR"
    fi

    # Identify if the extracted file match one of the expected archive version (V1, V2 ...)
    if [ -f "$_EXTRACTED_TEST_FILEDIR_V2" ]; then
      _USE_VERSION="2"
      _EXTRACTED_DIR="$_EXTRACTED_DIR_V2"
      _EXTRACTED_TEST_FILEDIR="$_EXTRACTED_TEST_FILEDIR_V2"
    elif [ -f "$_EXTRACTED_TEST_FILEDIR_V1" ]; then
      _USE_VERSION="1"
      _EXTRACTED_DIR="$_EXTRACTED_DIR_V1"
      _EXTRACTED_TEST_FILEDIR="$_EXTRACTED_TEST_FILEDIR_V1"
    else
      # If extraction is not valid, then delete the downloaded file so it can be tried again.
      _USE_VERSION=""
      if [ $i -lt 2 ]; then
        warn_user "Failed to extract binary. Trying to re-download again"
      fi
      rm -rf "$_EXTRACT_DIR" >/dev/null 2>&1
      rm -rf "$_DOWNLOAD_FILEPATH" >/dev/null 2>&1
    fi

    if [ -n "$_USE_VERSION" ]; then
      # Cleanup cache now that we have likely an older version to get rid of.
      cleanup_cache_as_needed "$_WORKDIR"
      break # Exit the retry loop.
    fi
  done

  # Do a final check that the extracted files are OK.
  if [ ! -f "$_EXTRACTED_TEST_FILEDIR" ]; then
    setup_error "Failed to download or extract precompiled binary for $_BRANCH"
  fi

  # Success
  PRECOMP_REMOTE_DOWNLOAD_DIR="$_EXTRACTED_DIR"
  PRECOMP_REMOTE_FILE_NAME_VERSION="$_USE_VERSION"
}
export -f download_PRECOMP_REMOTE

install_PRECOMP_REMOTE() {
  local _WORKDIR="$1"

  # This assume download_PRECOMP_REMOTE() was successfully completed before.
  if [ "$PRECOMP_REMOTE" != "true" ] || [ -z "$PRECOMP_REMOTE_DOWNLOAD_DIR" ]; then
    echo "PRECOMP_REMOTE=$PRECOMP_REMOTE"
    echo "PRECOMP_REMOTE_DOWNLOAD_DIR=$PRECOMP_REMOTE_DOWNLOAD_DIR"
    setup_error "Could not install precompiled binary for $_WORKDIR"
  fi

  # Detect if a previous build was done, if yes then "cargo clean".
  if [ -d "$SUI_REPO_DIR/target/debug/build" ] || [ -d "$SUI_REPO_DIR/target/release/build" ]; then
    (if cd "$SUI_REPO_DIR"; then cargo clean; else setup_error "Unexpected missing $SUI_REPO_DIR"; fi)
    # Deprecated cd_sui_log_dir
  fi

  # Create an array of "sui", "sui-tool"
  local _BINARIES=("sui" "sui-tool" "sui-faucet" "sui-node" "sui-test-validator" "sui-indexer")

  # Iterate the BINARIES array and copy/install the binaries.
  # Note: Although the binaries are 'release' we install also
  #       in the debug directory to make it 'easier' to find
  #       for any app.
  for _BIN in "${_BINARIES[@]}"; do
    local _SRC
    if [ "$PRECOMP_REMOTE_FILE_NAME_VERSION" = "1" ]; then
      _SRC="$PRECOMP_REMOTE_DOWNLOAD_DIR/$_BIN-$PRECOMP_REMOTE_PLATFORM-$PRECOMP_REMOTE_ARCH"
    else
      _SRC="$PRECOMP_REMOTE_DOWNLOAD_DIR/$_BIN"
    fi
    local _DST="$WORKDIRS/$_WORKDIR/sui-repo/target/debug/$_BIN"
    # Copy/install files when difference detected.
    copy_on_bin_diff "$_SRC" "$_DST"
    _DST="$WORKDIRS/$_WORKDIR/sui-repo/target/release/$_BIN"
    copy_on_bin_diff "$_SRC" "$_DST"
  done
}
export -f install_PRECOMP_REMOTE

copy_on_bin_diff() {
  local _SRC="$1"
  local _DST="$2"
  # Copy the file _SRC to _DST if the files are binary different.
  # If _DST does not exist, then copy to create it.
  # If _SRC does not exists, then do nothing.
  if [ ! -f "$_SRC" ]; then
    return
  fi
  local _DO_COPY=false
  if [ ! -f "$_DST" ]; then
    _DO_COPY=true
  else
    if ! cmp --silent "$_SRC" "$_DST"; then
      _DO_COPY=true
    fi
  fi
  if [ "$_DO_COPY" = "true" ]; then
    # Create the path/directories of the _DST as needed.
    mkdir -p "$(dirname "$_DST")"
    \cp -f "$_SRC" "$_DST"
  fi
}
export -f copy_on_bin_diff

export USER_GITHUB_TOKEN
update_USER_GITHUB_TOKEN_var() {
  # Use a GITHUB_TOKEN if set as env or from suibase.yaml
  if [ -n "$GITHUB_TOKEN" ]; then
    USER_GITHUB_TOKEN="$GITHUB_TOKEN"
  elif [ -n "${CFG_github_token:?}" ] && [ "${CFG_github_token:?}" != "~" ]; then
    USER_GITHUB_TOKEN="${CFG_github_token:?}"
  else
    USER_GITHUB_TOKEN=""
  fi
}
export -f update_USER_GITHUB_TOKEN_var

export HOST_PLATFORM="" # "uname -s" like output e.g. "Linux", "Darwin"...
export HOST_ARCH=""     # "uname -m" like output e.g. x86_64, arm64...
update_HOST_vars() {
  # Can be called multiple time and will cache the results.
  if [ -z "$HOST_PLATFORM" ]; then
    HOST_PLATFORM=$(uname -s)
    HOST_ARCH=$(uname -m)
    # For MacOS only while running in Rosetta context:
    # uname -m returns "x86_64" even if running on M processors.
    # Have to double check with an alternative. More info:
    #     https://stackoverflow.com/questions/65259300/detect-apple-silicon-from-command-line
    if [ "$HOST_PLATFORM" = "Darwin" ]; then
      if [ "$HOST_ARCH" != "arm64" ]; then
        local _CHECK_ALT
        _CHECK_ALT=$(sysctl -n machdep.cpu.brand_string | grep -o "Apple M")
        if [ -n "$_CHECK_ALT" ] && [[ $_CHECK_ALT == *"Apple M"* ]]; then
          HOST_ARCH="arm64"
        fi
      fi
    fi

    # Detect unsupported PLATFORM/ARCH combination and display debug info.
    local _UNSUPPORTED=false
    if [ "$HOST_PLATFORM" = "Linux" ]; then
      if [ "$HOST_ARCH" != "x86_64" ]; then
        _UNSUPPORTED=true
      fi
    else
      if [ "$HOST_PLATFORM" = "Darwin" ]; then
        if [ "$HOST_ARCH" != "x86_64" ] && [ "$HOST_ARCH" != "arm64" ]; then
          _UNSUPPORTED=true
        fi
      else
        _UNSUPPORTED=true
      fi
    fi

    if [ "$_UNSUPPORTED" = true ]; then
      setup_error "Unsupported platform [$HOST_PLATFORM] and arch [$HOST_ARCH]"
    fi
  fi
}
export -f update_HOST_vars

is_wsl() {
  if [ -f "/proc/sys/fs/binfmt_misc/WSLInterop" ] || [ -n "$WSL_DISTRO_NAME" ] || [[ "$(uname -r)" == *"WSL"* ]]; then
    true
    return
  fi
  false
  return
}
export -f is_wsl

has_param() {
  local _SHORT_OPT="$1"
  local _LONG_OPT="$2"
  # Initialize params with remaining parameters (exclude $1 and $2)
  local _PARAMS=("${@:3}")

  # If found, return true.
  for _PARAM in "${_PARAMS[@]}"; do
    if [ -z "$_PARAM" ]; then
      # Should not happen... but just in case one of $_SHORT_OPT or $_LONG_OPT is empty.
      continue
    fi

    if [[ "$_PARAM" == "$_SHORT_OPT" || "$_PARAM" == "$_LONG_OPT" ]]; then
      true
      return
    fi
  done

  false
  return
}
export -f has_param

export SUIBASE_DAEMON_UPGRADING=false
progress_suibase_daemon_upgrading() {
  SUIBASE_DAEMON_UPGRADING=true
  mkdir -p "$SUIBASE_TMP_DIR"
  touch "$SUIBASE_TMP_DIR/suibase-daemon-upgrading"
}
export -f progress_suibase_daemon_upgrading

exit_if_deps_missing() {
  # MacOs does not have flock normally installed.
  # If missing, then try to install it. Exit if not successful.
  update_HOST_vars
  if [ "$HOST_PLATFORM" = "Darwin" ]; then
    if ! which flock >/dev/null 2>&1; then
      if which brew >/dev/null 2>&1; then
        brew install flock >/dev/null 2>&1
      fi
      if ! which flock >/dev/null 2>&1; then
        setup_error "Must install flock. Try 'brew install flock'"
      fi
    fi
  fi
}

trig_daemons_refresh() {
  if command -v notify_suibase_daemon_fs_change >/dev/null 2>&1; then
    notify_suibase_daemon_fs_change
  fi

  if command -v notify_dtp_daemon_fs_change >/dev/null 2>&1; then
    notify_dtp_daemon_fs_change
  fi

  if command -v notify_suibase_daemon_workdir_change >/dev/null 2>&1; then
    notify_suibase_daemon_workdir_change
  fi
}
export -f trig_daemons_refresh

stop_all_services() {
  #
  # Exit if fails to get ALL the process stopped.
  #
  # The suibase-daemon and dtp-daemon are exception to the rule... they
  # "self-exit" when no longer needed.
  #
  # Returns:
  #   0: Success (all process needed to be stopped were stopped)
  #   1: Everything already stopped. Call was NOOP (except for user_request writing)

  # Note: Try hard to keep the dependency here low on $WORKDIR.
  #       We want to try to stop the processes even if most of
  #       the workdir content is in a bad state.
  local _OLD_USER_REQUEST
  if [ -d "$WORKDIRS/$WORKDIR" ]; then
    _OLD_USER_REQUEST=$(get_key_value "$WORKDIR" "user_request")
    # Always write to "touch" the file and possibly cause
    # downstream resynch/fixing.
    set_key_value "$WORKDIR" "user_request" "stop"
    if [ "$_OLD_USER_REQUEST" != "stop" ]; then
      sync_client_yaml
    fi
  fi

  if [ "${CFG_network_type:?}" = "remote" ]; then
    # Nothing needed to be stop for remote network.
    if [ "$_OLD_USER_REQUEST" = "stop" ]; then
      # Was already stopped.
      return 1
    fi
    # Transition to "stop" state successful.
    return 0
  fi

  if [ -z "$SUI_FAUCET_PROCESS_PID" ] && [ -z "$SUI_PROCESS_PID" ]; then
    return 1
  fi

  # Stop the processes in reverse order.
  if [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
    stop_sui_faucet_process
  fi

  if [ -n "$SUI_PROCESS_PID" ]; then
    stop_sui_process
  fi

  # Check if successful.
  if [ -z "$SUI_FAUCET_PROCESS_PID" ] && [ -z "$SUI_PROCESS_PID" ]; then
    echo "$WORKDIR now stopped"
  else
    setup_error "Failed to stop everything. Try again. Use \"$WORKDIR status\" to see what is still running."
  fi

  # Success. All process that needed to be stopped were stopped.
  trig_daemons_refresh
  return 0
}
export -f stop_all_services

start_all_services() {
  #
  # Exit if fails to get one of the needed process running.
  #
  # Returns:
  #   0: Success (all process needed to be started were started)
  #   1: Everything needed particular to this workdir already running
  #      (Note: suibase-daemon and dtp-daemon are not *particular* to a workdir)
  #
  local _OLD_USER_REQUEST
  _OLD_USER_REQUEST=$(get_key_value "$WORKDIR" "user_request")

  set_key_value "$WORKDIR" "user_request" "start"

  # A good time to double-check if some commands from the suibase.yaml need to be applied.
  copy_private_keys_yaml_to_keystore "$WORKDIRS/$WORKDIR/config/sui.keystore"

  # Also a good time to double-check the daemons are running (when needed).
  if ! start_suibase_daemon_as_needed; then
    setup_error "$SUIBASE_DAEMON_NAME taking too long to start? Check \"$WORKDIR status\" in a few seconds. If persisting, may be try to start again or upgrade with  ~/suibase/update?"
  fi

  if ! start_dtp_daemon_as_needed; then
    setup_error "$DTP_DAEMON_NAME taking too long to start? Check \"$WORKDIR status\" in a few seconds. If persisting, may be try to start again or upgrade with  ~/suibase/update?"
  fi

  # Verify if all other expected process are running.

  if [ "${CFG_network_type:?}" = "remote" ]; then
    # No other process expected for remote network.
    # Just check that suibase-daemon is responding.
    sync_client_yaml
    trig_daemons_refresh
    wait_for_json_rpc_up "${WORKDIR_NAME}"
    return 0
  fi

  # Verify if the faucet is supported for this version.
  local _SUPPORT_FAUCET
  if version_less_than "$SUI_VERSION" "sui 0.27" || [ "${CFG_sui_faucet_enabled:?}" != "true" ]; then
    _SUPPORT_FAUCET=false
  else
    _SUPPORT_FAUCET=true
  fi

  local _ALL_RUNNING=true
  if [ "$_SUPPORT_FAUCET" = true ] && [ -z "$SUI_FAUCET_PROCESS_PID" ]; then
    _ALL_RUNNING=false
  fi

  if [ -z "$SUI_PROCESS_PID" ]; then
    _ALL_RUNNING=false
  fi

  if [ "$_ALL_RUNNING" = true ]; then
    sync_client_yaml
    trig_daemons_refresh
    wait_for_json_rpc_up "${WORKDIR_NAME}"
    return 1
  fi


  if [ -z "$SUI_PROCESS_PID" ]; then
    # Note: start_sui_process has to call sync_client_yaml itself to remove the
    #       use of the proxy. This explains why start_sui_process is called on
    #       the exit of this function and not before.
    start_sui_process
  fi

  if [ -z "$SUI_PROCESS_PID" ]; then
    setup_error "Not started or taking too long to start? Check \"$WORKDIR status\" in a few seconds. If persisting down, may be try again or \"$WORKDIR update\" of the code?"
  fi

  if $_SUPPORT_FAUCET; then
    if [ -z "$SUI_FAUCET_PROCESS_PID" ]; then
      start_sui_faucet_process
    fi

    if [ -z "$SUI_FAUCET_PROCESS_PID" ]; then
      setup_error "Faucet not started or taking too long to start? Check \"$WORKDIR status\" in a few seconds. If persisting down, may be try again or \"$WORKDIR update\" of the code?"
    fi
  fi

  # Success. All process that needed to be started were started.
  sync_client_yaml
  trig_daemons_refresh
  wait_for_json_rpc_up "${WORKDIR_NAME}"
  return 0
}

is_at_least_one_service_running() {
  # Keep this function cohesive with start/stop
  #
  # SUIBASE_DAEMON and DTP_DAEMON are exceptions to the rule... they should always run!
  update_SUI_FAUCET_PROCESS_PID_var
  update_SUI_PROCESS_PID_var
  if [ -n "$SUI_FAUCET_PROCESS_PID" ] || [ -n "$SUI_PROCESS_PID" ]; then
    true
    return
  fi
  false
  return
}
export -f is_at_least_one_service_running
