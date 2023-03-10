#!/bin/bash

# Do not call this script directly. It is a "common script" sourced by other sui-base scripts.
#
# It initializes a bunch of environment variable, verify that some initialization took
# place, identify some common user errors etc...

# Sui-base does not work with version below these.
MIN_SUI_VERSION="sui 0.27.0"
MIN_RUST_VERSION="rustc 1.65.0"

# Mandatory command line:
#    $1 : Should be the "$0" of the caller script.
#    $2 : Should be the workdir string (e.g. "active", "localnet"... )
SCRIPTS_DIR="$(dirname $1)"
SCRIPT_NAME="$(basename $1)"
WORKDIR="$2"

# Initialize variables driven by $WORKDIR
case $WORKDIR in
  localnet)
    SUI_REPO_BRANCH="devnet"
    SUI_SCRIPT="lsui"
    ;;
  devnet)
    SUI_REPO_BRANCH="devnet"
    SUI_SCRIPT="dsui"
    ;;
  testnet)
    SUI_REPO_BRANCH="testnet"
    SUI_SCRIPT="tsui"
    ;;
  active)
    SUI_REPO_BRANCH="NULL"
    SUI_SCRIPT="asui"
    ;;
  *)
    echo "globals: not supported workdir [$WORKDIR]"
    ;;
esac

# Utility functions.
setup_error() { echo "$*" 1>&2 ; exit 1; }
export -f setup_error
version_greater_equal() { printf '%s\n%s\n' "$2" "$1" | sort --check=quiet --version-sort; }
export -f version_greater_equal
script_cmd() { script -efqa "$SCRIPT_OUTPUT" -c "$*"; }
export -f script_cmd
beginswith() { case $2 in "$1"*) true;; *) false;; esac; }
export -f beginswith

# Two very convenient variables for directories.
SUI_BASE_DIR="$HOME/sui-base"
WORKDIRS="$SUI_BASE_DIR/workdirs"

# Some other commonly used locations.
LOCAL_BIN="$HOME/.local/bin"

SUI_REPO_DIR="$WORKDIRS/$WORKDIR/sui-repo"
SUI_BIN_DIR="$SUI_REPO_DIR/target/debug"

CONFIG_DATA_DIR="$WORKDIRS/$WORKDIR/config"
PUBLISHED_DATA_DIR="$CONFIG_DATA_DIR/published-data"

# Configuration files (often needed for sui CLI calls)
NETWORK_CONFIG="$CONFIG_DATA_DIR/network.yaml"
CLIENT_CONFIG="$CONFIG_DATA_DIR/client.yaml"

# This is the default repo for localnet/devnet/testnet scripts.
# Normally $SUI_REPO_DIR will symlink to $SUI_REPO_DIR_DEFAULT
SUI_REPO_DIR_DEFAULT="$WORKDIRS/$WORKDIR/sui-repo-default"

# Location for genesis data for "default" repo.
DEFAULT_GENESIS_DATA_DIR="$SCRIPTS_DIR/genesis_data"

# Location for generated genesis data (on first start after set-sui-repo)
GENERATED_GENESIS_DATA_DIR="$WORKDIRS/$WORKDIR/genesis-data"

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
    # If not already done, initialize the default repo.
    if [ ! -d "$SUI_REPO_DIR_DEFAULT" ]; then
      git clone -b devnet https://github.com/MystenLabs/sui.git "$SUI_REPO_DIR_DEFAULT"  || setup_error "Failed getting Sui $SUI_REPO_BRANCH branch from github";
      set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT";
    fi

    # Add back the default sui-repo link in case its was deleted.
    if [ ! -L "$SUI_REPO_DIR" ]; then
      set_sui_repo_dir "$SUI_REPO_DIR_DEFAULT";
    fi

    # Update sui devnet local repo (if needed)
    (cd "$SUI_REPO_DIR" && git remote update >& /dev/null)
    V1=$(cd "$SUI_REPO_DIR"; git rev-parse HEAD)
    V2=$(cd "$SUI_REPO_DIR"; git rev-parse '@{u}')
    if [ "$V1" != "$V2" ]
    then
      # Does a bit more than needed, but should allow to recover
      # from most operator error...
      echo Updating sui $WORKDIR in sui-base...
      (cd "$SUI_REPO_DIR" && git switch $SUI_REPO_BRANCH > /dev/null)
      (cd "$SUI_REPO_DIR" && git fetch > /dev/null)
      (cd "$SUI_REPO_DIR" && git reset --hard origin/$SUI_REPO_BRANCH > /dev/null)
      (cd "$SUI_REPO_DIR" && git merge '@{u}')
    fi
    echo "Building $WORKDIR using latest Sui $SUI_REPO_BRANCH branch..."
  fi

  (cd "$SUI_REPO_DIR"; cargo build)

  # Sanity test that the sui binary works
  if [ ! -f "$SUI_BIN_DIR/sui" ]; then
    setup_error "$SUI_BIN_DIR/sui binary not found"
  fi

  update_SUI_VERSION_var;

  # Check if sui is recent enough.
  version_greater_equal "$SUI_VERSION" "$MIN_SUI_VERSION" || setup_error "Sui binary version too old (not supported)"
}

export -f build_sui_repo_branch

check_dev_setup() {
  # Sanity check the setup was completed successfully.
  #
  # This should be check for most scripts.
  #
  # This is to minimize support/confusion. First, get the initial setup right
  # before letting the user do more damage...
  if [ ! -d "$WORKDIRS" ]; then
    setup_error "$WORKDIRS missing. Please run '$WORKDIR update' first"
  fi

  if [ ! -d "$SUI_REPO_DIR" ]; then
    setup_error "$SUI_REPO_DIR missing. Please run '$WORKDIR update' first"
  fi

  if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
    setup_error "$WORKDIRS/$WORKDIR missing. Please run '$WORKDIR update' first"
  fi

  if [ ! -f "$NETWORK_CONFIG" ]; then
    setup_error "$NETWORK_CONFIG missing. Please run '$WORKDIR update' first"
  fi

  if [ ! -f "$CLIENT_CONFIG" ]; then
    setup_error "$CLIENT_CONFIG missing. Please run '$WORKDIR update' first"
  fi
}
export -f check_dev_setup

is_localnet_installed() {

  # Just check if present on the filesystem, not if running or executeable.
  # Detect if any problem. Return true only if installation is likely healthy.
  #
  # That one is different than check_dev_setup because we do not want to
  # report error, we just want to detect and fix automatically.
  if [ ! -d "$WORKDIRS" ]; then
    false; return;
  fi

  if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
    false; return;
  fi

  if [ ! -d "$SUI_REPO_DIR" ]; then
    false; return;
  fi

  if [ ! -d "$CONFIG_DATA_DIR" ]; then
    false; return;
  fi

  if [ ! -f "$NETWORK_CONFIG" ]; then
    false; return;
  fi

  if [ ! -f "$CLIENT_CONFIG" ]; then
    false; return;
  fi

  if [ ! -f "$SUI_BIN_DIR/sui" ]; then
    false; return;
  fi

  true; return;
}
export -f is_localnet_installed

common_create_workdirs() {
  mkdir -p "$WORKDIRS"

  if [ "$WORKDIR" != "active" ]; then
    mkdir -p "$WORKDIRS/$WORKDIR"
  fi

  # Create the sui-exec file (if does not exists)
  if [ ! -f "$WORKDIRS/$WORKDIR/sui-exec" ]; then
    cp "$SCRIPTS_DIR/common/__sui-exec.sh" "$WORKDIRS/$WORKDIR/sui-exec"
  fi

  # Check if there is an active symlink, if not, create one.
  # (Note: if the symlink is broken, do not attempt to fix it.
  #  The symlink represent the "user intent" and must be
  #  preserved when it exists).
  if [ ! -L "$WORKDIRS/active" ]; then
     set_active_workdir;
  fi
}
export -f common_create_workdirs

set_active_workdir() {
  # Create a symlink to the current $WORKDIR if not already done.
  update_ACTIVE_WORKDIR_var;
  if [ ! -L "$WORKDIRS/active" ]; then
    ln -s "$WORKDIRS/$WORKDIR" "$WORKDIRS/active"
  else
    if [[ "$ACTIVE_WORKDIR" != "$WORKDIR" ]]; then
      ln -sfT "$WORKDIRS/$WORKDIR" "$WORKDIRS/active"
    fi
  fi
}
export -f set_active_workdir

update_ACTIVE_WORKDIR_var() {
  # This is the active $WORKDIR (deduced from the symlink).
  if [ ! -L $WORKDIRS/active ]; then
    unset ACTIVE_WORKDIR
  else
    RESOLVED_PATH="$(readlink -f $WORKDIRS/active)"
    ACTIVE_WORKDIR="$(basename $RESOLVED_PATH)"
  fi
}
export -f update_ACTIVE_WORKDIR_var

update_SUI_PROCESS_PID_var() {
  # Useful to check if the sui process is running (this is the parent for the "localnet")
  #
  # Details on the cryptic parsing:
  #   Get ps with "sui start" in its command line, grep exclude itself from the list, head takes the first process (should
  #   not be more than one) the 1st sed remove leading space, the 2nd sed split words into line and finally the pid is the
  #   word on the first/head line.
  #
  if [[ $(uname) == "Darwin" ]]; then
    SUI_PROCESS_PID=$(ps x -o pid,comm | grep "sui" | grep -v grep | head -n 1 | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | head -n 1)
  else
    SUI_PROCESS_PID=$(ps x -o pid,cmd | grep "sui start" | grep -v grep | head -n 1 | sed -e 's/^[[:space:]]*//' | sed 's/ /\n/g' | head -n 1)
  fi
}
export -f update_SUI_PROCESS_PID_var

update_SUI_VERSION_var() {
  # Take note that $SUI_BIN_DIR here is used to properly consider if the
  # context of the script is localnet, devnet, testnet, mainet... (they
  # are not the same binaries and versions).
  SUI_VERSION=$($SUI_BIN_DIR/sui --version)
  if [ -z "$SUI_VERSION" ]; then
    setup_error "$SUI_BIN_DIR/sui --version not running as expected"
  fi
}
export -f update_SUI_VERSION_var

stop_sui_process() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUI_PROCESS_PID_var;
  if [ ! -z "$SUI_PROCESS_PID" ]; then
    echo "Stopping $WORKDIR (process pid $SUI_PROCESS_PID)"
    if [[ $(uname) == "Darwin" ]]; then
      kill -9 $SUI_PROCESS_PID
    else
      skill -9 $SUI_PROCESS_PID
    fi

    # Make sure it is dead.
    end=$((SECONDS+15))
    DEAD=false
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

    if [ ! -z "$SUI_PROCESS_PID" ]; then
      setup_error "Sui process pid=$SUI_PROCESS_PID still running. Try again, or stop (kill) the sui process yourself before proceeding."
    fi
  fi
}
export -f stop_sui_process

start_sui_process() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already started.

  # Detect an installation problem (took a while to debug when it did happen)
  if [ ! -f "$NETWORK_CONFIG" ]; then
    setup_error "$NETWORK_CONFIG missing. Please re-run '$WORKDIR update' to fix."
  fi

  if [ ! -f "$CLIENT_CONFIG" ]; then
    setup_error "$CLIENT_CONFIG missing. Please re-run '$WORKDIR update' to fix."
  fi

  update_SUI_PROCESS_PID_var;
  if [ -z "$SUI_PROCESS_PID" ]; then
    echo "Starting localnet process"
    $SUI_BIN_DIR/sui start --network.config "$NETWORK_CONFIG" >& "$CONFIG_DATA_DIR/sui-process.log" &
    NEW_SUI_PID=$!

    # Loop until "sui client" confirms to be working, or exit if that takes
    # more than 30 seconds.
    end=$((SECONDS+30))
    ALIVE=false
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      CHECK_ALIVE=$($SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" objects | grep -i Digest)
      if [ ! -z "$CHECK_ALIVE" ]; then
        ALIVE=true
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

update_SUI_REPO_INFO_var() {
  # This is intended for display to user (human).
  BRANCH_NAME = $(cd $SUI_REPO_DIR; git branch --show-current)
  if is_sui_repo_dir_default; then
    SUI_REPO_INFO="git branch is [$BRANCH_NAME]"
  else
    RESOLVED_SUI_REPO=$(readlink $SUI_REPO_DIR)
    RESOLVED_SUI_REPO_BASENAME=$(basename "$RESOLVE_SUI_REPO")
    SUI_REPO_INFO="git branch is [$BRANCH_NAME], sui-repo set to [$RESOLVED_SUI_REPO_BASENAME]"
  fi
}
export -f update_SUI_REPO_INFO_var

ensure_client_OK() {
  # This is just in case the user switch the envs on the clients instead of simply using
  # the scripts... we have then to fix things up here. Not an error unless the fix fails.

  # TODO Add paranoiac validation, fix the URL part, for now this is used only for localnet.

  # Make sure localnet exists in sui envs (ignore errors because likely already exists)
  $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" new-env --alias $WORKDIR --rpc http://0.0.0.0:9000 >& /dev/null

  # Make localnet the active envs (should already be done, just in case, do it again here).
  $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" switch --env $WORKDIR > /dev/null
}
export -f ensure_client_OK

publish_clear_output() {
  if [ -n "$MOVE_TOML_PACKAGE_NAME" ]; then
    rm -rf "$PUBLISH_DATA_DIR/$MOVE_TOML_PACKAGE_NAME/package_id.txt"
  fi
  # Following files created only on confirmed success of publication.
  #rm -rf "$PUBLISH_DATA_DIR/client_addresses.txt"
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

  if [ -f $1/Move.toml ]; then
    MOVE_TOML_DIR=$1
  else
    if [ -f $1/move/Move.toml ]; then
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

publish_localnet() {

  PASSTHRU_OPTIONS="$@"

  ensure_client_OK;

  if [ -z "$MOVE_TOML_PACKAGE_NAME" ]; then
    echo "Package name could not be found"
    exit
  fi

  INSTALL_DIR="$PUBLISHED_DATA_DIR/$MOVE_TOML_PACKAGE_NAME"

  echo "Package name=[$MOVE_TOML_PACKAGE_NAME]"
  #echo "Build location=[$INSTALL_DIR]"
  mkdir -p "$INSTALL_DIR"

  # Set the output for the "script_cmd"
  SCRIPT_OUTPUT="$INSTALL_DIR/publish-output.txt"
  rm -rf $SCRIPT_OUTPUT

  # Run unit tests.
  #script_cmd "lsui move test --install-dir \"$INSTALL_DIR\" -p \"$MOVE_TOML_DIR\""

  # Build the Move package for publication.
  #echo Now publishing on network
  CMD="lsui client publish --gas-budget 30000 --install-dir \"$INSTALL_DIR\" \"$MOVE_TOML_DIR\" $PASSTHRU_OPTIONS --json 2>&1 1>$INSTALL_DIR/publish-output.json"

  echo $CMD
  echo Publishing...
  script_cmd $CMD;

  #  TODO Investigate problem with exit status here...

  # Grab the first packageId line after the "events:" and "publish:" string have been met (in order).
  # Will be a problem if can publish multiple package at same time, json format change etc.
  # Certainly not perfect, but good enough for today...
  #
  # Details on cryptic sed line (it is multiple steps, each seperated by a ';')
  #   Remove everything until '"events":'', everything until '"publish":'', all quotes and all commas.
  #   Then tr remove whitespace (keeps newlines) and grep the first packageId line met (head -1)
  #
  ID_LINE=$(cat $INSTALL_DIR/publish-output.json | sed '1,/\"events\":/d; 1,/\"publish\":/d; s/\"//g; s/,//g' | tr -d "[:blank:]" | grep packageId | head -1)

  # echo "id_line=[$ID_LINE]"

  if [ -z "$ID_LINE" ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "Could not find the package id from $SCRIPT_OUTPUT"
  fi

  # Extract first hexadecimal literal found.
  # Define the seperator (IFS) as the JSON ':'
  ID=""
  IFS=":"
  for i in $ID_LINE
  do
    if beginswith 0x $i; then
      ID=$i
      break;
    fi
  done

  # Best-practice to revert IFS to default.
  unset IFS

  echo "ID=[$ID]"

  if [ -z "$ID" ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "Could not find Package id in $SCRIPT_OUTPUT"
  fi

  # Test the publication by retreiving object information from the network
  # using that parsed package id.
  script_cmd "lsui client object $ID"
  echo Verifying client can access new package on network...
  validation=$(lsui client object $ID | grep -i "package")
  if [ -z "$validation" ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "Unexpected object type (Not a package)"
  fi
  JSON_STR="[\"$ID\"]"
  echo $JSON_STR > "$INSTALL_DIR/package-id.json"

  echo "Package ID is $JSON_STR"
  echo "Also written in [$INSTALL_DIR/package-id.json]"
  echo Publication Successful
}
export -f publish_localnet


# Verify if $SUI_REPO_DIR symlink is toward a user repo (not default).
#
# false if the symlink does not exist.
#
# (Note: does not care if the target directory exists).
is_sui_repo_dir_override() {
  # Verify if symlink resolves and is NOT toward the default.
  if [ -L "$SUI_REPO_DIR" ]; then
    RESOLVED_SUI_REPO_DIR=$(readlink $SUI_REPO_DIR)
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
    echo "$WORKDIR using default local sui repo"
  fi
}
export -f set_sui_repo_dir_default

set_sui_repo_dir() {

  OPTIONAL_PATH="$@"

  # User errors?
  if [ ! -d "$OPTIONAL_PATH" ]; then
    setup_error "Path [$OPTIONAL_PATH] not found"
  fi

  # The -T is important because target is a directory and without it
  # the command line arguments would be interpreted in the 3rd form
  # described in "man ln".
  ln -sfT "$OPTIONAL_PATH" "$SUI_REPO_DIR"

  # Verify success.
  if is_sui_repo_dir_default; then
    echo "$WORKDIR using default local sui repo [$OPTIONAL_PATH]"
  else
    if is_sui_repo_dir_override; then
      echo "$WORKDIR set-sui-repo is now [$OPTIONAL_PATH]"
    else
      setup_error "$WORKDIR set-sui-repo failed [$OPTIONAL_PATH]";
    fi
  fi
}
export -f set_sui_repo_dir

sui_exec() {

  # Display some sui-base related info if called without any parameters.
  DISPLAY_SUI_BASE_HELP=false
  if [ $# -eq 0 ]; then
    DISPLAY_SUI_BASE_HELP=true
  fi

  # Quick sanity check that sui-base was properly installed.
  check_dev_setup;

  # Use the proper config automatically.
  SUI_SUBCOMMAND=$1

  LAST_ARG="${@: -1}"
  if [[ "$LAST_ARG" == "--help" || "$LAST_ARG" == "-h" ]]; then
    DISPLAY_SUI_BASE_HELP=true
  fi

  if [[ $SUI_SUBCOMMAND == "client" || $SUI_SUBCOMMAND == "console" ]]; then
    shift 1
    $SUI_BIN_DIR/sui $SUI_SUBCOMMAND --client.config "$CLIENT_CONFIG" "$@"

    # Print a friendly warning if localnet sui process found not running.
    # Might help explain weird error messages...
    if [ "$DISPLAY_SUI_BASE_HELP" = false ]; then
      update_SUI_PROCESS_PID_var;
      if [ -z "$SUI_PROCESS_PID" ]; then
        echo
        echo "Warning: localnet not running"
        echo "Do 'localnet start' to get it started."
      fi
    fi
    exit
  fi

  if [[ $SUI_SUBCOMMAND == "network" ]]; then
    shift 1
    $SUI_BIN_DIR/sui $SUI_SUBCOMMAND --network.config "$NETWORK_CONFIG" "$@"
    exit
  fi

  if [[ $SUI_SUBCOMMAND == "genesis" ]]; then
    # Protect the user from damaging its localnet
    if [[ "$2" == "--help" || "$2" == "-h" ]]; then
      $SUI_BIN_DIR/sui genesis --help
    fi
    echo
    setup_error "Use sui-base 'localnet start' script instead"
  fi

  if [[ $SUI_SUBCOMMAND == "start" ]]; then
    # Protect the user from starting more than one sui process.
    if [[ "$2" == "--help" || "$2" == "-h" ]]; then
      $SUI_BIN_DIR/sui start --help
    fi
    echo
    setup_error "Use sui-base 'localnet start' script instead"
  fi

  # Are you getting an error : The argument '--keystore-path <KEYSTORE_PATH>' was provided
  # more than once, but cannot be used multiple times?
  #
  # This is because by default lsui point to the keystore created with the localnet.
  #
  # TODO Fix this. Still default to workdirs, but allow user to override with its own --keystore-path.
  #
  if [[ $SUI_SUBCOMMAND == "keytool" ]]; then
    shift 1
    $SUI_BIN_DIR/sui $SUI_SUBCOMMAND --keystore-path "$CONFIG_DATA_DIR/sui.keystore" "$@"
    exit
  fi

  # By default, just pass transparently everything to the proper sui binary.
  $SUI_BIN_DIR/sui "$@"

  if [ "$DISPLAY_SUI_BASE_HELP" = true ]; then
    update_ACTIVE_WORKDIR_var;
    if [ -n "$ACTIVE_WORKDIR" ]; then
      echo
      echo "$ACTIVE_WORKDIR is set-active for asui"
    fi
  fi
}
export -f sui_exec
