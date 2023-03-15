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
SCRIPT_PATH="$(dirname $1)"
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
  cargobin)
    SUI_REPO_BRANCH="NULL"
    SUI_SCRIPT="csui"
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

# Two key directories location.
SUI_BASE_DIR="$HOME/sui-base"
WORKDIRS="$SUI_BASE_DIR/workdirs"

# Some other commonly used locations.
LOCAL_BIN="$HOME/.local/bin"
SCRIPTS_DIR="$SUI_BASE_DIR/scripts"
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

# Now load all the $CFG_ variables from the sui-base.yaml files.
source "$SCRIPTS_DIR/common/__parse-yaml.sh"
update_sui_base_yaml() {
  # Load defaults.
  YAML_FILE="$SCRIPTS_DIR/defaults/$WORKDIR/sui-base.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval $(parse_yaml "$YAML_FILE" "CFG_")
  fi

  # Load overrides from workdir.
  YAML_FILE="$WORKDIRS/$WORKDIR/sui-base.yaml"
  if [ -f "$YAML_FILE" ]; then
    eval $(parse_yaml "$YAML_FILE" "CFG_")
  fi
}
export -f update_sui_base_yaml

update_sui_base_yaml;

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

is_workdir_initialized() {
  # Just check if enough present on the filesystem to allow configuration, not
  # if there is enough for running the sui client.
  #
  # In other word, detect if at least the "create" command was performed.
  #
  # This function is different than check_workdir_ok which is more exhaustive
  # and requires healthy client binaries.
  if [ ! -d "$WORKDIRS" ]; then
    false; return;
  fi

  if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
    false; return;
  fi

  if [ ! -f "$WORKDIRS/$WORKDIR/sui-base.yaml" ]; then
    false; return;
  fi

  if [ ! -L "$WORKDIRS/$WORKDIR/config" ]; then
    false; return;
  fi

  true; return
}
export -f is_workdir_initialized

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
    $(cmp --silent "$SCRIPTS_DIR/templates/$FILENAME" "$WORKDIRS/$WORKDIR_PARAM/$FILENAME" ) || {
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

create_workdir_as_needed() {
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

  if [ "$WORKDIR_PARAM" = "cargobin" ]; then
    create_config_symlink_as_needed "$WORKDIR_PARAM" "$HOME/.sui/sui_config"
  fi

  if [ "$WORKDIR_PARAM" = "devnet" ] || [ "$WORKDIR_PARAM" = "localnet" ]; then
    create_config_symlink_as_needed "$WORKDIR_PARAM" "$WORKDIRS/$WORKDIR_PARAM/config-default"
  fi

}
export -f create_workdir_as_needed

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
  #if [ "$CFG_network_type" = "local" ]; then
    # Make sure localnet exists in sui envs (ignore errors because likely already exists)
    #echo $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" new-env --alias $WORKDIR --rpc http://0.0.0.0:9000
    $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" new-env --alias $WORKDIR --rpc http://0.0.0.0:9000 >& /dev/null

    # Make localnet the active envs (should already be done, just in case, do it again here).
    #echo $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" switch --env $WORKDIR
    $SUI_BIN_DIR/sui client --client.config "$CLIENT_CONFIG" switch --env $WORKDIR > /dev/null
  #fi
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

  # The -n is important because target is a directory and without it
  # the command line arguments would be interpreted in the 3rd form
  # described in "man ln".
  ln -nsf "$OPTIONAL_PATH" "$SUI_REPO_DIR"

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
  create_workdir_as_needed "cargobin"

  if [ "$workdir_was_missing" = true ]; then
    if [ -d "$WORKDIRS/cargobin" ]; then
      echo "Created workdir for existing ~/.cargo/bin/sui client"
    else
      echo "Warning: workdir creation for ~/.cargo/bin/sui client failed."
    fi
  fi
}
export -f create_cargobin_as_needed
