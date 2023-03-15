#!/bin/bash

# You must source __globals.sh before __workdir-exec.sh

# workdir_exec() is the key "public" function of this file.

# One command always expected from the user.
CMD_START_REQ=false
CMD_STOP_REQ=false
CMD_STATUS_REQ=false
CMD_CREATE_REQ=false
CMD_UPDATE_REQ=false
CMD_REGEN_REQ=false
CMD_PUBLISH_REQ=false
CMD_SET_ACTIVE_REQ=false
CMD_SET_SUI_REPO_REQ=false

usage_local() {
  echo "Usage: $WORKDIR [COMMAND] <Options>"
  echo
  echo "  Simulate a sui network running fully on this machine"
  echo "  Accessible from http://0.0.0.0:9000"
  echo
  echo "COMMAND:"
  echo
  echo "   start:   start $WORKDIR (sui process will run in background)"
  echo "   stop:    stop $WORKDIR (sui process will exit)"
  echo "   status:  indicate if running or not"
  echo
  echo "   create:  Create workdir only. This can be useful for changing"
  echo "            the configuration before doing the first start."
  echo
  echo "   update:  Update local sui repo and regen $WORKDIR."
  echo "            Note: Will not do any git operations if your own"
  echo "                  repo is configured with set-sui-repo."
  echo
  echo "   regen:   Only regenerate $WORKDIR. Useful for gas refueling."
  echo
  echo "   publish: Publish the module specified in the Move.toml found"
  echo "            in current directory or optional '--path <path>'"
  echo
  echo "   set-active:"
  echo "            Makes $WORKDIR the active context for many"
  echo "            development tools and the 'asui' script."
  echo
  echo "   set-sui-repo:"
  echo "            Allows to specify a '--path <path>' to use your own"
  echo "            local repo instead of the default latest from github."
  echo "            Just omit '--path' to return to default."
  echo


}

usage_remote() {
  echo "Usage: $WORKDIR [COMMAND] <Options>"
  echo
  echo "  Sui-base $WORKDIR workdir to interact with a remote Sui network"
  echo
  echo "COMMAND:"
  echo
  echo "   start:   start $WORKDIR sui-base services (runs in background)"
  echo "   stop:    stop all $WORKDIR sui-base services"
  echo "   status:  indicate if services running and network accessible."
  echo
  echo "   create:  Create workdir only. This can be useful for changing"
  echo "            the configuration before doing the first start."
  echo
  echo "   update:  Update local sui repo and build client binary."
  echo "            Note: Will not do any git operations if your own"
  echo "                  repo is configured with set-sui-repo."
  echo
  echo "   publish: Publish module specified in the Move.toml found"
  echo "            in current directory or optional '--path <path>'"
  echo
  echo "   set-active:"
  echo "            Makes $WORKDIR the active context for many"
  echo "            development tools and the 'asui' script."
  echo
  echo "   set-sui-repo:"
  echo "            Allows to specify a '--path <path>' to use your own"
  echo "            local repo instead of the default latest from github."
  echo "            Just omit '--path' to return to default."
  echo
}

usage() {
  if [ "$CFG_network_type" = "local" ]; then
    usage_local;
  else
    usage_remote;
  fi

  # Quick check if installed, then help the user about the location.
  if [ -d "$HOME/sui-base/workdirs" ]; then
    echo "All sui-base outputs are in ~/sui-base/workdirs/$WORKDIR"
  fi

  exit
}

echo_help_on_not_initialized() {
    echo "$WORKDIR workdir not initialized"

    if is_sui_repo_dir_default; then
      echo
      echo "Do \"$WORKDIR start\" to use default latest Sui repo (recommended)"
      echo
      echo "Check \"$WORKDIR --help\" for more advanced configuration"
    else
      echo
      echo "Do \"$WORKDIR start\" to initialize"
    fi
}

workdir_exec() {

  case "$1" in
    start) CMD_START_REQ=true ;;
    stop) CMD_STOP_REQ=true ;;
    status) CMD_STATUS_REQ=true ;;
    create) CMD_CREATE_REQ=true ;;
    update) CMD_UPDATE_REQ=true ;;
    regen) CMD_REGEN_REQ=true ;;
    publish) CMD_PUBLISH_REQ=true ;;
    set-active) CMD_SET_ACTIVE_REQ=true ;;
    set-sui-repo) CMD_SET_SUI_REPO_REQ=true ;;
    *) usage;;
  esac

  shift # Consume the command.

  # Optional params (the "debug" is purposely not documented).
  DEBUG_RUN=false

  while [[ "$#" -gt 0 ]]; do
    case $1 in
        # -t|--target) target="$2"; shift ;; That's an example with a parameter
        # -f|--flag) flag=1 ;; That's an example flag

        -d|--debug) DEBUG_RUN=true ;;

        -p|--path)
           # see: https://stackoverflow.com/questions/9018723/what-is-the-simplest-way-to-remove-a-trailing-slash-from-each-parameter
           OPTIONAL_PATH="${2%/}"; shift
           if [ -z "$OPTIONAL_PATH" ]; then
             echo "--path <path> must be specified"
             exit
           fi
           ;;
        *)
        if [ "$CMD_PUBLISH_REQ" = true ]; then
          case $1 in
            --json) echo "--json option superfluous. JSON always generated on publish by sui-base. See publish-output.json." ;;
            --install-dir) echo "Do no specify --install-dir when publishing with sui-base. Output is always in published-data location instead." ;;
            *) PASSTHRU_OPTIONS="$PASSTHRU_OPTIONS $1" ;;
          esac
        else
          echo "Unknown parameter passed: $1"; exit 1
        fi ;;
    esac
    shift
  done

  if [ "$DEBUG_RUN" = true ]; then
    echo "Debug flag set. Will run Localnet in foreground Ctrl-C to Exit"
  fi

  # Detect invalid COMMAND and Option combinations.

  # Check if '-p <path>'' is used with a valid subcommand
  if [ -n "$OPTIONAL_PATH" ]; then
    if [ "$CMD_PUBLISH_REQ" = true ] || [ "$CMD_SET_SUI_REPO_REQ" = true ]; then
      # Validate if the path exists.
      if [ ! -d "$OPTIONAL_PATH" ]; then
        echo "Path [$OPTIONAL_PATH] not found"
        exit
      fi
    else
      echo "-p <path> option not valid with this command";
      exit
    fi
  fi

  if [ "$CFG_network_type" = "local" ]; then
    is_local=true
  else
    is_local=false
  fi

  # First, take care of the easy "status" command that does not touch anything.

  if $is_local; then
    update_SUI_PROCESS_PID_var;
  fi

  update_ACTIVE_WORKDIR_var;

  if [ "$CMD_STATUS_REQ" = true ]; then
    if is_workdir_initialized; then
      if $is_local; then
        if [ -z "$SUI_PROCESS_PID" ]; then
          echo -e "localnet \033[1;31mSTOPPED\033[0m"
        else
          echo -e "localnet \033[1;32mRUNNING\033[0m (process pid $SUI_PROCESS_PID)"
        fi
      fi

      update_SUI_VERSION_var;
      echo "$SUI_VERSION"
      #update_SUI_REPO_INFO_var;
      #echo "$SUI_VERSION ($SUI_REPO_INFO)"
      DISPLAY_AS_WARNING=true
      DISPLAY_FIELD="$ACTIVE_WORKDIR"
      if [ "$ACTIVE_WORKDIR" = "$WORKDIR" ]; then
        DISPLAY_AS_WARNING=false
      fi

      if [ -z "$DISPLAY_FIELD" ]; then
        DISPLAY_FIELD="<none>"
        DISPLAY_AS_WARNING=true
      fi

      if [ "$DISPLAY_AS_WARNING" = true ]; then
        echo -e "asui selection: \033[1;33m$DISPLAY_FIELD\033[0m"
      else
        echo -e "asui selection: $DISPLAY_FIELD"
      fi

      if is_sui_repo_dir_override; then
        echo "set-sui-repo: [$RESOLVED_SUI_REPO_DIR]"
      fi
    else
      echo_help_on_not_initialized;
    fi
    exit
  fi

  # Second, take care of the case that just stop/start the localnet.
  if [ "$CMD_START_REQ" = true ]; then
    if is_workdir_initialized; then
      if $is_local; then
        if [ "$SUI_PROCESS_PID" ]; then
          echo "localnet already running (process pid $SUI_PROCESS_PID)"
          update_SUI_VERSION_var;
          echo "$SUI_VERSION"
        else
          start_sui_process;
        fi
      else
        echo "$WORKDIR installed (no process needed to be further started)"
      fi
      exit
    fi
    # Note: If workdir not installed, keep going to install it.
  fi

  if [ "$CMD_STOP_REQ" = true ]; then
    if ! $is_local; then
      echo "Not applicable yet for $WORKDIR (work in progress)"
      exit
    fi

    if is_workdir_initialized; then
      if [ "$SUI_PROCESS_PID" ]; then
        stop_sui_process;
        # Confirm result (although stop_sui_process may have handled error already)
        update_SUI_PROCESS_PID_var;
        if [ "$SUI_PROCESS_PID" ]; then
          setup_error "Failed to stop localnet"
        else
          echo "localnet now stopped"
        fi
      else
        echo "localnet already stopped"
      fi
    else
      echo_help_on_not_initialized;
    fi
    exit
  fi

  if [ "$CMD_PUBLISH_REQ" = true ]; then

    if ! $is_local; then
      echo "Not implement yet for $WORKDIR (work in progress)"
      exit
    fi

    if [ -n "$OPTIONAL_PATH" ]; then
      update_MOVE_TOML_DIR_var $OPTIONAL_PATH;
    else
      update_MOVE_TOML_DIR_var $PWD;
    fi

    if [ -z $MOVE_TOML_DIR ]; then
      echo "\"$WORKDIR publish\" must have Move.toml in current directory or --path specified"
    fi

    if is_workdir_initialized; then
      # publication requires localnet to run.
      # If stopped, then try (once) to start it.
      update_SUI_PROCESS_PID_var;
      if [ "$SUI_PROCESS_PID" ]; then
        publish_localnet $PASSTHRU_OPTIONS;
      else
        start_sui_process;
        if [ "$SUI_PROCESS_PID" ]; then
          publish_localnet $PASSTHRU_OPTIONS;
        else
          echo "Unable to start localnet"
        fi
      fi
    else
      echo_help_on_not_initialized;
    fi
    exit
  fi

  if [ "$CMD_SET_ACTIVE_REQ" = true ]; then
    if is_workdir_initialized; then
      if [ "$ACTIVE_WORKDIR" = "$WORKDIR" ]; then
        echo "$WORKDIR is already active"
      else
        echo "Making $WORKDIR active"
        set_active_symlink_force "$WORKDIR";
      fi
    else
      echo_help_on_not_initialized;
    fi
    exit
  fi

  # Detect user action that should be NOOP.
  if [ "$CMD_SET_SUI_REPO_REQ" = true ] && [ -z "$OPTIONAL_PATH" ]; then
    if is_sui_repo_dir_default; then
      setup_error "$WORKDIR already using default repo. no change."
    fi
  fi

  if [ "$CMD_CREATE_REQ" = true ]; then
    # Check for what is minimally needed for configuration.
    if is_workdir_initialized; then
      setup_error "$WORKDIR already created."
    fi
  fi

  # Finally, take care of the more complicated cases that involves
  # git, workdir/config creation and genesis.
  create_workdir_as_needed "$WORKDIR"; # Create/repair $WORKDIR

  if [ "$CMD_CREATE_REQ" = true ]; then
    # No further action when "create" command.
    echo "$WORKDIR created"
    exit
  fi

  # The script should not be called from a location that could get deleted.
  # It would work (on Linux) because of reference counting, but it could
  # lead to some confusion for the user.

  if $is_local; then
    CWD=$(pwd -P)
    if [[ "$CWD" = "$CONFIG_DATA_DIR_DEFAULT"* ]]; then
      echo "This script should not be called from a location that could be deleted [$CONFIG_DATA_DIR]."
      setup_error "Change current directory location and try again."
    fi

    if [[ "$CWD" = "$PUBLISHED_DATA_DIR"* ]]; then
      echo "This script should not be called from a location that could be deleted [$PUBLISHED_DATA_DIR]."
      setup_error "Change current directory location and try again."
    fi

    # Stop localnet (noop if not running)
    stop_sui_process;

    # Clean-up previous localnet (if exists)
    RM_DIR="$CONFIG_DATA_DIR_DEFAULT"
    if [ -d "$RM_DIR" ]; then
      echo "Clearing existing localnet data"
      rm -rf "$RM_DIR"
    fi

    # Delete localnet publish directory (if exists) to force re-publication.
    RM_DIR="$PUBLISH_DATA_DIR"
    if [ -d "$RM_DIR" ]; then
      rm -rf "$RM_DIR"
    fi
  fi

  if [ "$CMD_SET_SUI_REPO_REQ" = true ]; then
    if $is_local; then
      update_SUI_PROCESS_PID_var;
      if [ "$SUI_PROCESS_PID" ]; then
        # Force to stop. Otherwise the running process and config will be out-of-sync.
        setup_error "Can't change config while $WORKDIR running. Do \"$WORKDIR stop\"."
      fi
    fi

    if [ -z "$OPTIONAL_PATH" ]; then
      set_sui_repo_dir_default;
    else
      set_sui_repo_dir "$OPTIONAL_PATH";
    fi

    if $is_local; then
      # Clean-up generated genesis data because did succesfully switch repo.
      RM_DIR="$GENERATED_GENESIS_DATA_DIR"
      if [ -d "$RM_DIR" ]; then
        rm -rf "$RM_DIR"
      fi
    fi

    exit
  fi

  # Create and build the sui-repo.
  # Should not download on a regen or set-sui-repo, but still need to do "cargo build" in case the
  # binary are not up to data (or done yet).
  ALLOW_DOWNLOAD="true" # Using string because passing outside as param
  if [ "$CMD_REGEN_REQ" = true ]; then
    ALLOW_DOWNLOAD="false"
  fi
  if is_sui_repo_dir_override; then
    ALLOW_DOWNLOAD="false"
  fi

  build_sui_repo_branch "$ALLOW_DOWNLOAD";

  if $is_local; then
    source "$HOME/sui-base/scripts/common/__workdir-init-local.sh"
    workdir_init_local;
  else
    source "$HOME/sui-base/scripts/common/__workdir-init-remote.sh"
    workdir_init_remote;
  fi

  if $is_local; then
    # Start the new localnet normally.
    start_sui_process;
    echo "========"
  fi

  ensure_client_OK;

  # print sui envs to help debugging (if someone else is using this script).
  $SUI_EXEC client envs
  echo "========"

  if $is_local; then
    echo "All client addresses with coins:"
  else
    echo "All client addresses:"
  fi

  $SUI_EXEC client addresses
  echo "========"

  WALLET_ADDR=$($SUI_EXEC client active-address)
  echo "Coins owned by $WALLET_ADDR (active):"
  $SUI_EXEC client gas

  # TODO Display only if a shortcut is defined.
  echo "----------------------------------------------------------------------"
  echo
  echo "Remember:"
  echo "  Use \"$SUI_SCRIPT\" to access your $WORKDIR"
  echo
  echo "Success. Try it by typing \"$SUI_SCRIPT client gas\""
}
