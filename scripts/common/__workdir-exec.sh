#!/bin/bash

# You must source __globals.sh before __workdir-exec.sh

# workdir_exec() is the key "public" function of this file.

# One command always expected from the user.
CMD_START_REQ=false
CMD_STOP_REQ=false
CMD_STATUS_REQ=false
CMD_CREATE_REQ=false
CMD_DELETE_REQ=false
CMD_UPDATE_REQ=false
CMD_REGEN_REQ=false
CMD_PUBLISH_REQ=false
CMD_LINKS_REQ=false
CMD_SET_ACTIVE_REQ=false
CMD_SET_SUI_REPO_REQ=false
CMD_FAUCET_REQ=false

usage_local() {
  echo_low_green "$WORKDIR"
  echo "  suibase $SUI_BASE_VERSION"
  echo
  echo "  Workdir to simulate a Sui network running fully on this machine"
  echo "  Accessible from $CFG_links_1_rpc"
  echo
  echo "  If not sure what to do, then type '$WORKDIR start' and all that is"
  echo "  needed will be downloaded, built and started for you."
  echo
  echo_low_yellow "USAGE: "
  echo
  echo "      $WORKDIR <SUBCOMMAND> <Options>"
  echo
  echo_low_yellow "SUBCOMMANDS:"
  echo
  echo
  echo_low_green "   start"
  echo "    Start $WORKDIR processes (will run in background)"
  echo_low_green "   stop"
  echo "     Stop $WORKDIR processes (will all exit)"
  echo_low_green "   status"
  echo "   Info about all the suibase related processes"
  echo
  echo_low_green "   create"
  echo "   Create workdir only. This can be useful for changing"
  echo "            the configuration before doing the first build/start."
  echo
  echo_low_green "   build"
  echo "    Download/update local repo and build binaries only. You may"
  echo "            have to do next an update, start or regen to create a wallet."
  echo
  echo_low_green "   delete"
  echo "   Delete workdir completely. Can free up a lot of"
  echo "            disk space for when the localnet is not needed."
  echo
  echo_low_green "   update"
  echo "   Update local sui repo and perform a regen."
  echo "            Note: When set-sui-repo is configured, there is no git"
  echo "                  operations and this does regen only."
  echo
  echo_low_green "   regen"
  echo "    Build binaries, create a wallet as needed and rebuild the"
  echo "            network. Useful for gas refueling or to wipe-out the network"
  echo "            'database' on binaries update or when suspecting problems."
  echo
  echo_low_green "   publish"
  echo "  Publish the module specified in the Move.toml found"
  echo "            in current directory or optional '--path <path>'"
  echo
  echo_low_green "   faucet"
  echo "   Get new coins toward any address."
  echo "            Do \"$WORKDIR faucet\" for more info"
  echo
  echo_low_green "   links"
  echo "   Get RPC server statistics."
  echo
  echo_low_green "   set-active"
  echo
  echo "            Makes $WORKDIR the active context for many"
  echo "            development tools and the 'asui' script."
  echo
  echo_low_green "   set-sui-repo"
  echo
  echo "            Allows to specify a '--path <path>' to use your own"
  echo "            local repo instead of the default latest from github."
  echo "            Just omit '--path' to return to default."
  echo
}

usage_remote() {
  echo_low_green "$WORKDIR"
  echo "  suibase $SUI_BASE_VERSION"
  echo
  echo "  Workdir to interact with a remote Sui network"
  echo
  echo_low_yellow "USAGE: "
  echo
  echo "      $WORKDIR <SUBCOMMAND> <Options>"
  echo
  echo "  If not sure what to do, then type '$WORKDIR start' and all that is"
  echo "  needed will be downloaded, built and started for you."
  echo
  echo_low_yellow "SUBCOMMANDS:"
  echo
  echo
  echo_low_green "   start"
  echo "    Start $WORKDIR processes (will run in background)"
  echo_low_green "   stop"
  echo "     Stop $WORKDIR processes (will all exit)"
  echo_low_green "   status"
  echo "   Info about all the suibase related processes"
  echo
  echo_low_green "   create"
  echo "   Create workdir only. This can be useful for changing"
  echo "            the configuration before doing the first build/start."
  echo
  echo_low_green "   build"
  echo "    Download/update local repo and build binaries only. You"
  echo "            may have to do next an update or start to create a wallet."
  echo
  echo_low_green "   update"
  echo "   Update local sui repo, build binaries, create a wallet as"
  echo "            needed."
  echo "            Note: Will not do any git operations if your own"
  echo "                  repo is configured with set-sui-repo."
  echo
  echo_low_green "   publish"
  echo "  Publish the module specified in the Move.toml found"
  echo "            in current directory or optional '--path <path>'"
  echo
  echo_low_green "   links"
  echo "    Get RPC server statistics."
  echo
  echo_low_green "   set-active"
  echo
  echo "            Makes $WORKDIR the active context for many"
  echo "            development tools and the 'asui' script."
  echo
  echo_low_green "   set-sui-repo"
  echo
  echo "            Allows to specify a '--path <path>' to use your own"
  echo "            local repo instead of the default latest from github."
  echo "            Just omit '--path' to return to default."
  echo
}

usage() {
  if [ "${CFG_network_type:?}" = "local" ]; then
    usage_local
  else
    usage_remote
  fi

  # Quick check if installed, then help the user about the location.
  if [ -d "$SUIBASE_DIR/workdirs" ]; then
    echo "All suibase outputs are in ~/suibase/workdirs/$WORKDIR"
  fi

  exit
}

workdir_exec() {

  exit_if_not_installed

  CMD_REQ=$1
  shift # Consume the command.

  case "$CMD_REQ" in
  start) CMD_START_REQ=true ;;
  stop) CMD_STOP_REQ=true ;;
  status) CMD_STATUS_REQ=true ;;
  create) CMD_CREATE_REQ=true ;;
  build) CMD_BUILD_REQ=true ;;
  delete) CMD_DELETE_REQ=true ;;
  update) CMD_UPDATE_REQ=true ;;
  regen) CMD_REGEN_REQ=true ;;
  publish) CMD_PUBLISH_REQ=true ;;
  links) CMD_LINKS_REQ=true ;;
  set-active) CMD_SET_ACTIVE_REQ=true ;;
  set-sui-repo) CMD_SET_SUI_REPO_REQ=true ;;
  faucet) CMD_FAUCET_REQ=true ;;
  *) usage ;;
  esac

  # Optional params (the "debug" and "nobinary" are purposely not documented).
  DEBUG_RUN=false
  NOBINARY=false
  JSON_PARAM=false

  # Parsing the command line shifting "rule":
  #   -t|--target) target="$2"; shift ;; That's an example with a parameter
  #   -f|--flag) flag=1 ;; That's an example flag

  case "$CMD_REQ" in
  faucet)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_RUN=true ;;
      *) PASSTHRU_OPTIONS="$PASSTHRU_OPTIONS $1" ;;
      esac
      shift
    done
    ;; # End parsing faucet
  set-sui-repo)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_RUN=true ;;
      -p | --path)
        # see: https://stackoverflow.com/questions/9018723/what-is-the-simplest-way-to-remove-a-trailing-slash-from-each-parameter
        OPTIONAL_PATH="${2%/}"
        shift
        if [ -z "$OPTIONAL_PATH" ]; then
          echo "--path <path> must be specified"
          exit 1
        fi
        ;;
      *)
        echo "Unknown parameter passed: $1"
        exit 1
        ;;
      esac
      shift
    done
    ;; # End parsing publish
  publish)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_RUN=true ;;
      --nobinary) NOBINARY=true ;;
      -p | --path)
        OPTIONAL_PATH="${2%/}"
        shift
        if [ -z "$OPTIONAL_PATH" ]; then
          echo "--path <path> must be specified"
          exit 1
        fi
        ;;
      --json) echo "--json option superfluous. JSON always generated on publish by suibase. See publish-output.json." ;;
      --install-dir) echo "Do no specify --install-dir when publishing with suibase. Output is always in published-data location instead." ;;
      *) PASSTHRU_OPTIONS="$PASSTHRU_OPTIONS $1" ;;
      esac
      shift
    done
    ;; # End parsing publish
  build)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug | --nobinary)
        echo "Option '$1' not compatible with build command"
        exit 1
        ;;
      *) PASSTHRU_OPTIONS="$PASSTHRU_OPTIONS $1" ;;
      esac
      shift
    done
    ;; # End parsing build
  links)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_RUN=true ;;
      --json) JSON_PARAM=true ;;
      *)
        echo "Unknown parameter passed: $1"
        exit 1
        ;;
      esac
      shift
    done
    ;; # End parsing links
  *)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_RUN=true ;;
      --nobinary) NOBINARY=true ;;
      *)
        echo "Unknown parameter passed: $1"
        exit 1
        ;;
      esac
      shift
    done
    ;; # End parsing default cases
  esac

  if [ "$DEBUG_RUN" = true ]; then
    echo "Debug flag set. May run in foreground Ctrl-C to Exit"
  fi

  if [ "$NOBINARY" = true ]; then
    echo "nobinary flag set. Will not build sui binaries from the repo."
  fi

  # Validate if the path exists.
  if [ -n "$OPTIONAL_PATH" ]; then
    if [ ! -d "$OPTIONAL_PATH" ]; then
      echo "Path [ $OPTIONAL_PATH ] not found"
      exit
    fi
  fi

  ###################################################################
  #
  #  Detect if a repair is needed related to the suibase renaming
  #  (this should be removed pass mid-2024 or so...)
  #
  ####################################################################
  if [ -f "$WORKDIRS/$WORKDIR/sui-base.yaml" ]; then
    # shellcheck source=SCRIPTDIR/../../repair
    source "$SUIBASE_DIR"/repair
  fi

  if [ "$CFG_network_type" = "local" ]; then
    is_local=true
  else
    is_local=false
  fi

  # Detect commands that should not be done on a remote network.
  if [ "$is_local" = false ]; then
    case "$CMD_REQ" in
    regen)
      echo "Command '$CMD_REQ' not allowed for $WORKDIR"
      exit 1
      ;;
    esac
  fi

  ###################################################################
  #
  #  Most command line validation done (PASSTHRU_OPTIONS remaining)
  #
  #  Source more files and do actual work from this point.
  #
  ####################################################################

  if $is_local; then
    # shellcheck source=SCRIPTDIR/__sui-faucet-process.sh
    source "$SUIBASE_DIR/scripts/common/__sui-faucet-process.sh"
    update_SUI_FAUCET_PROCESS_PID_var

    update_SUI_PROCESS_PID_var
  fi

  update_ACTIVE_WORKDIR_var

  # shellcheck source=SCRIPTDIR/__suibase-daemon.sh
  source "$SUIBASE_DIR/scripts/common/__suibase-daemon.sh"
  update_SUIBASE_DAEMON_PID_var

  # First, take care of the easier "status" and "links" that are "read only" commands.

  if [ "$CMD_STATUS_REQ" = true ]; then
    exit_if_workdir_not_ok
    exit_if_sui_binary_not_ok

    local _USER_REQUEST
    _USER_REQUEST=$(get_key_value "user_request")

    update_SUI_VERSION_var

    # Verify from suibase.yaml if proxy services are expected.
    # If yes, then populate STATUS/INFO.
    local _SUPPORT_PROXY
    local _MLINK_STATUS
    local _MLINK_INFO
    if [ "${CFG_proxy_enabled:?}" == "false" ]; then
      _SUPPORT_PROXY=false
    else
      _SUPPORT_PROXY=true
      unset JSON_RESP
      get_suibase_daemon_status "data"
      update_JSON_VALUE "code" "$JSON_RESP"
      if [ -n "$JSON_VALUE" ]; then
        update_JSON_VALUE "message" "$JSON_RESP"
        if [ -n "$JSON_VALUE" ]; then
          _MLINK_STATUS="DOWN"
          _MLINK_INFO="$JSON_RESP"
        fi
      fi
      if [ -z "$_MLINK_STATUS" ]; then
        update_JSON_VALUE "status" "$JSON_RESP"
        if [ -n "$JSON_VALUE" ]; then
          _MLINK_STATUS="$JSON_VALUE"
        fi
        update_JSON_VALUE "info" "$JSON_RESP"
        if [ -n "$JSON_VALUE" ]; then
          _MLINK_INFO="$JSON_VALUE"
        fi
      fi
    fi

    if $is_local; then
      update_SUI_FAUCET_VERSION_var

      # Verify if the faucet is supported for this version.
      local _SUPPORT_FAUCET
      if version_less_than "$SUI_VERSION" "sui 0.27" || [ "${CFG_sui_faucet_enabled:?}" != "true" ]; then
        _SUPPORT_FAUCET=false
      else
        _SUPPORT_FAUCET=true
      fi

      # Overall status: STOPPED or OK/DEGRADED/DOWN
      echo -n "localnet "
      if [ "$_USER_REQUEST" = "stop" ]; then
        echo_red "STOPPED"

      else
        if [ -z "$SUI_PROCESS_PID" ]; then
          echo_red "DOWN"
        else
          local _DEGRADED=false
          if $_SUPPORT_FAUCET && [ -z "$SUI_FAUCET_PROCESS_PID" ]; then
            _DEGRADED=true
          fi
          if $_SUPPORT_PROXY && [ -z "$SUIBASE_DAEMON_PID" ]; then
            _DEGRADED=true
          fi
          if [ "$_DEGRADED" = true ]; then
            echo_yellow "DEGRADED"
          else
            echo_green "OK"
          fi
        fi
      fi
      echo

      # Individual process status
      if [ "$_USER_REQUEST" = "stop" ]; then
        # Show process "abnormally" still running.
        if [ -n "$SUI_PROCESS_PID" ] || [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
          echo "---"
          if [ -n "$SUI_PROCESS_PID" ]; then
            echo "localnet process : STILL RUNNING (pid $SUI_PROCESS_PID)"
          fi
          if [ -n "$SUI_FAUCET_PROCESS_PID" ]; then
            echo "faucet process   : STILL RUNNING (pid $SUI_FAUCET_PROCESS_PID)"
          fi
        fi
      else
        echo "---"
        echo_process "localnet process" true "$SUI_PROCESS_PID"
        echo_process "faucet process" "$_SUPPORT_FAUCET" "$SUI_FAUCET_PROCESS_PID"
      fi
    fi

    if ! $is_local; then
      # Overall status: STOPPED or OK/DEGRADED/DOWN
      echo -n "$WORKDIR "
      if [ "$_USER_REQUEST" = "stop" ]; then
        echo -n "services "
        echo_red "STOPPED"
        echo
      else
        echo_green "OK"
        echo
        echo "---"
      fi
    fi

    # Append the information common to all.
    if [ ! "$_USER_REQUEST" = "stop" ]; then
      _INFO=$(
        echo -n "http://"
        echo_blue "0.0.0.0"
        echo -n ":"
        echo_blue "${CFG_proxy_port_number:?}"
      )
      echo_process "proxy server" "$_SUPPORT_PROXY" "$SUIBASE_DAEMON_PID" "$_INFO"
    fi

    if [ "$_SUPPORT_PROXY" = true ]; then
      echo -n "multi-link RPC   : "
      case $_MLINK_STATUS in
      "OK")
        echo_blue "OK"
        ;;
      "DOWN")
        echo_red "DOWN"
        ;;
      esac
      echo " ( $_MLINK_INFO )"
    fi
    echo "---"
    echo -n "client version: "
    echo_blue "$SUI_VERSION"
    echo

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

    echo -n "asui selection: [ "
    if [ "$DISPLAY_AS_WARNING" = true ]; then
      echo_yellow "$DISPLAY_FIELD"
    else
      echo_blue "$DISPLAY_FIELD"
    fi
    echo " ]"

    if is_sui_repo_dir_override; then
      echo "set-sui-repo  : $RESOLVED_SUI_REPO_DIR"
    fi

    # Detect the situation where everything is NOT RUNNING, but
    # yet the user_request is 'start'... and there are clear indication
    # that the system was rebooted (because /tmp directories are missing).
    if [ "$is_local" == "true" ] && [ "$_USER_REQUEST" = "start" ]; then
      # TODO Check if the last time the $_USER_REQUEST was written
      #      prior to the last reboot time.
      #
      # Note "uptime -s" not working on macOS, otherwise just comparing the
      # following would work great:
      # date -r ~/suibase/workdirs/localnet/.state/user_request "+%Y-%m-%d %H:%M:%S"
      # uptime -s
      if [ -z "$SUI_PROCESS_PID" ] &&
        [ -z "$FAUCET_PROCESS_PID" ] &&
        [ -z "$SUIBASE_DAEMON_PID" ]; then
        if [ ! -d "$SUIBASE_TMP_DIR" ]; then
          warn_user "Looks like the system was rebooted. Please do '$WORKDIR start' to resume."
        fi
      fi
    fi
    exit
  fi

  if [ "$CMD_LINKS_REQ" = true ]; then
    exit_if_workdir_not_ok

    if [ "${CFG_proxy_enabled:?}" == "false" ]; then
      echo "You must enable monitoring of $WORKDIR RPC servers for this command to work."
      echo
      echo "Add 'proxy_enabled: true' to either:"
      echo "      ~/suibase/workdirs/$WORKDIR/suibase.yaml"
      echo "  or  ~/suibase/workdirs/common/suibase.yaml"
      echo
      echo "More info: https://suibase.io/how-to/proxy"
      exit 1
    fi

    # Display the stats from the proxy server.
    show_suibase_daemon_get_links "$DEBUG_RUN" "$JSON_PARAM"

    exit
  fi

  # Second, take care of the case that just stop/start processes.
  if [ "$CMD_START_REQ" = true ]; then

    if is_workdir_ok && is_sui_binary_ok; then

      # A good time to check if the user did mess up with the
      # workdir and fix potentially missing files.
      repair_workdir_as_needed "$WORKDIR"

      # Note: nobody should have tried to run the sui binary yet.
      # So this is why the update_SUI_VERSION_var need to be done here.
      update_SUI_VERSION_var

      start_all_services
      _RES=$?
      if [ "$_RES" -eq 1 ]; then
        if [ "$is_local" == "true" ]; then
          echo "$WORKDIR already running"
        else
          echo "$WORKDIR services already running"
        fi

        echo "$SUI_VERSION"
      fi

      if [ "$_RES" -eq 0 ]; then
        if [ "$is_local" == "false" ]; then
          # Remote networks do not have any local process to start (the
          # effect is only within the suibase-daemon), so put a little
          # feedback to acknowledge the user action.
          echo "$WORKDIR services started"
        fi
      fi

      exit
    fi
    # Note: If workdir/binary/config not OK, keep going to install or repair it.
  fi

  if [ "$CMD_STOP_REQ" = true ]; then
    stop_all_services
    _RES=$?
    if [ "$_RES" -eq 1 ]; then
      if [ "$is_local" == "true" ]; then
        echo "$WORKDIR already stopped"
      else
        echo "$WORKDIR services already stopped"
      fi
    fi
    if [ "$_RES" -eq 0 ]; then
      if [ "$is_local" == "false" ]; then
        # Remote networks do not have any local process to stop (the
        # effect is only within the suibase-daemon), so put a little
        # feedback to acknowledge the user action.
        echo "$WORKDIR services stopped"
      fi
    fi

    exit
  fi

  if [ "$CMD_FAUCET_REQ" = true ]; then
    exit_if_workdir_not_ok
    exit_if_sui_binary_not_ok

    # Verify that the faucet is supported for this version.
    if version_less_than "$SUI_VERSION" "sui 0.27"; then
      setup_error "faucet not supported for this older sui version"
    fi

    # Verify that the faucet is enabled and running.
    if [ "${CFG_sui_faucet_enabled:?}" != "true" ]; then
      setup_error "faucet feature disabled (see suibase.yaml )"
    fi

    start_all_services # Start the faucet as needed (and exit if fails).

    faucet_command "$PASSTHRU_OPTIONS"
    exit
  fi

  if [ "$CMD_PUBLISH_REQ" = true ]; then

    if [ -n "$OPTIONAL_PATH" ]; then
      update_MOVE_TOML_DIR_var "$OPTIONAL_PATH"
    else
      update_MOVE_TOML_DIR_var "$USER_CWD"
    fi

    if [ -z "$MOVE_TOML_DIR" ]; then
      echo "\"$WORKDIR publish\" must have Move.toml in current directory or --path specified"
    fi

    exit_if_workdir_not_ok
    exit_if_sui_binary_not_ok

    # shellcheck source=SCRIPTDIR/__publish.sh
    source "$SUIBASE_DIR/scripts/common/__publish.sh"

    if $is_local; then
      # publication requires localnet to run.
      # If stopped, then try (once) to start it.
      update_SUI_PROCESS_PID_var
      if [ "$SUI_PROCESS_PID" ]; then
        publish_local "$PASSTHRU_OPTIONS"
      else
        start_all_services
        if [ "$SUI_PROCESS_PID" ]; then
          publish_local "$PASSTHRU_OPTIONS"
        else
          error_exit "Unable to start $WORKDIR"
        fi
      fi
    else
      publish_local "$PASSTHRU_OPTIONS"
    fi
    exit
  fi

  if [ "$CMD_SET_ACTIVE_REQ" = true ]; then
    exit_if_workdir_not_ok

    if [ "$ACTIVE_WORKDIR" = "$WORKDIR" ]; then
      info_exit "$WORKDIR is already active"
    else
      set_active_symlink_force "$WORKDIR"
      info_exit "$WORKDIR is now active"
    fi
  fi

  # For now we support delete of localnet only.
  # Need to be more careful for testnet/devnet to preserve key.
  if [ "$CMD_DELETE_REQ" = true ]; then
    if ! $is_local; then
      info_exit "Not supported yet for $WORKDIR"
    fi

    if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
      info_exit "$WORKDIR is already deleted"
    fi

    echo "Deleting $WORKDIR"
    # shellcheck disable=SC2115
    rm -rf "$WORKDIRS/$WORKDIR"
    info_exit "$WORKDIR now deleted"
  fi

  # Detect user action that should be NOOP.
  if [ "$CMD_SET_SUI_REPO_REQ" = true ] && [ -z "$OPTIONAL_PATH" ]; then
    if is_sui_repo_dir_default; then
      info_exit "$WORKDIR already using default repo. no change."
    fi
  fi

  local _WORKDIR_WAS_OK
  if is_workdir_ok; then
    _WORKDIR_WAS_OK=true
  else
    _WORKDIR_WAS_OK=false
  fi

  # Finally, take care of the more complicated cases that involves
  # git, workdir/config creation and genesis.
  repair_workdir_as_needed "$WORKDIR" # Create/repair $WORKDIR

  if [ "$CMD_CREATE_REQ" = true ]; then
    # No further action when "create" command.
    if $_WORKDIR_WAS_OK; then
      # Note: did check for repair even if already created (just in case).
      info_exit "$WORKDIR already created"
    else
      info_exit "$WORKDIR created"
    fi
  fi

  if $is_local; then
    # The script should not be called from a location that could get deleted.
    # It would work (on Linux) because of reference counting, but it could
    # lead to some confusion for the user.
    local _DIR_CAN_GET_DELETED
    if [[ "$USER_CWD" = "$CONFIG_DATA_DIR_DEFAULT"* ]]; then
      _DIR_CAN_GET_DELETED=$CONFIG_DATA_DIR_DEFAULT
    fi

    if [[ "$USER_CWD" = "$PUBLISHED_DATA_DIR"* ]]; then
      _DIR_CAN_GET_DELETED=$PUBLISHED_DATA_DIR
    fi

    if [ -n "$_DIR_CAN_GET_DELETED" ]; then
      echo "This script should not be called from a location that could be deleted [$_DIR_CAN_GET_DELETED]."
      setup_error "Change current directory location and try again."
    fi

    # Stop all processes (noop if not running)
    stop_all_services

    # Clean-up previous localnet (if exists)
    RM_DIR="$CONFIG_DATA_DIR_DEFAULT"
    if [ -d "$RM_DIR" ]; then
      echo "Clearing existing localnet data"
      rm -rf "$RM_DIR"
      rm -rf "$WORKDIRS/$WORKDIR/.state/dns"
    fi

    # Delete localnet publish directory (if exists) to force re-publication.
    RM_DIR="$PUBLISHED_DATA_DIR"
    if [ -d "$RM_DIR" ]; then
      rm -rf "$RM_DIR"
    fi
  fi

  if [ "$CMD_SET_SUI_REPO_REQ" = true ]; then
    if $is_local; then
      # That should not be true at this point... but still... do a sanity check here.
      if is_at_least_one_service_running; then
        setup_error "Can't change repo while $WORKDIR running. Do \"$WORKDIR stop\"."
      fi
    fi

    local _DIR_CAN_GET_DELETED
    if [[ "$USER_CWD" = "$GENERATED_GENESIS_DATA_DIR"* ]]; then
      _DIR_CAN_GET_DELETED=$GENERATED_GENESIS_DATA_DIR
    fi
    if [ -n "$_DIR_CAN_GET_DELETED" ]; then
      echo "This script should not be called from a location that could be deleted [$_DIR_CAN_GET_DELETED]."
      setup_error "Change current directory location and try again."
    fi

    if [ -z "$OPTIONAL_PATH" ]; then
      set_sui_repo_dir_default
    else
      set_sui_repo_dir "$OPTIONAL_PATH"
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

  ALLOW_BUILD="true"
  if [ "$NOBINARY" = true ]; then
    ALLOW_BUILD="false"
  fi

  build_sui_repo_branch "$ALLOW_DOWNLOAD" "$ALLOW_BUILD" "$PASSTHRU_OPTIONS"

  if [ "$CMD_BUILD_REQ" = true ]; then
    # No further action needed when "build" command.
    local _BINARIES=()
    if [ -f "$SUI_BIN_DIR/sui" ]; then
      _BINARIES+=("sui")
    fi
    if [ -f "$SUI_BIN_DIR/sui-faucet" ]; then
      _BINARIES+=("sui-faucet")
    fi
    if [ -f "$SUI_BIN_DIR/sui-node" ]; then
      _BINARIES+=("sui-node")
    fi
    if [ ${#_BINARIES[@]} -gt 0 ]; then
      info_exit "Binaries available in '$SUI_BIN_DIR' are: ${_BINARIES[*]}"
    else
      info_exit "No binaries found in 'SUI_BIN_DIR'. Check for errors."
    fi
  fi

  if [ "$ALLOW_BUILD" = false ]; then
    # Can't do anything more than getting the repo.
    return
  fi

  if $is_local; then
    # shellcheck source=SCRIPTDIR/__workdir-init-local.sh
    source "$SUIBASE_DIR/scripts/common/__workdir-init-local.sh"
    workdir_init_local
  else
    # shellcheck source=SCRIPTDIR/__workdir-init-remote.sh
    source "$SUIBASE_DIR/scripts/common/__workdir-init-remote.sh"
    workdir_init_remote
  fi

  # This is the second pass with repair_workdir_as_needed. This is
  # done after regen to produce the .state/dns and whatever else
  # went missing.
  repair_workdir_as_needed "$WORKDIR" # Create/repair as needed

  # Start the local services (will be NOOP if already running).
  start_all_services
  echo "========"

  ensure_client_OK

  # print sui envs to help debugging (if someone else is using this script).

  CLIENT_YAML_ENVS=$($SUI_EXEC client envs | grep $WORKDIR)
  echo "SUI envs: $CLIENT_YAML_ENVS"
  echo "========"

  echo "All client addresses:"

  $SUI_EXEC client addresses
  echo "========"

  local _ADV="Try it by typing \"$SUI_SCRIPT client gas\""

  WALLET_ADDR=$($SUI_EXEC client active-address)
  if [[ "$WALLET_ADDR" == *"None"* ]]; then
    echo "There are no addresses in the wallet."
    _ADV="You can create the first wallet address with \"$SUI_SCRIPT client new-address ed25519\""
  else
    COINS_OWNED=$($SUI_EXEC client gas)
    COINS_SUM=$(echo "$COINS_OWNED" | awk "{ sum += \$3} END { print sum }")
    if [ "$COINS_SUM" = "0" ]; then
      echo "Coins owned by $WALLET_ADDR (active): None"
    else
      echo "Coins owned by $WALLET_ADDR (active):"
      echo "$COINS_OWNED"
      echo "----------------------------------------------------------------------------------"
    fi
  fi

  echo
  echo "Remember:"
  local _YOUR
  if $is_local; then
    _YOUR="your "
  fi
  echo "  Use \"$SUI_SCRIPT\" to access $_YOUR$WORKDIR"
  echo
  echo "Success. $_ADV"

  # Warn the user if the suibase.yaml default branch was overriden and the
  # actual branch is not the same. Recommend to do an update in that case.
  if is_sui_repo_dir_default; then
    if [ -d "$SUI_REPO_DIR_DEFAULT" ] && [ "${CFG_default_repo_branch:?}" != "${CFGDEFAULT_default_repo_branch:?}" ]; then
      # Check for mismatch.
      local _BRANCH_NAME
      _BRANCH_NAME=$(if cd "$SUI_REPO_DIR_DEFAULT"; then git branch --show-current; else echo "unknown"; fi)
      if [ "$_BRANCH_NAME" != "$CFG_default_repo_branch" ]; then
        warn_user "suibase.yaml is requesting for branch [$CFG_default_repo_branch] but the sui-repo is on [$_BRANCH_NAME]. Do '$WORKDIR update' to fix this."
      fi
    fi
  fi
}

stop_all_services() {
  #
  # Exit if fails to get ALL the process stopped.
  #
  # The suibase-daemon is an exception to the rule... it
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
    _OLD_USER_REQUEST=$(get_key_value "user_request")
    set_key_value "user_request" "stop"
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
  #      (Note: suibase-daemon is not *particular* to a workdir)
  local _OLD_USER_REQUEST
  _OLD_USER_REQUEST=$(get_key_value "user_request")

  set_key_value "user_request" "start"

  # A good time to double-check if some commands from the suibase.yaml need to be applied.
  copy_private_keys_yaml_to_keystore "$WORKDIRS/$WORKDIR/config/sui.keystore"

  # Also a good time to double-check the suibase-daemon is running (if needed).
  if ! start_suibase_daemon_as_needed; then
    setup_error "$SUIBASE_DAEMON_NAME taking too long to start? Check \"$WORKDIR status\" in a few seconds. If persisting, may be try to start again or upgrade with  ~/suibase/update?"
  fi

  # Verify if all other expected process are running.

  if [ "${CFG_network_type:?}" = "remote" ]; then
    # No other process expected for remote network.
    if [ "$_OLD_USER_REQUEST" = "start" ]; then
      # Was already started.
      return 1
    fi
    # Transition to "start" state successful.
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
    return 1
  fi

  # One or more other process need to be started.

  if [ -z "$SUI_PROCESS_PID" ]; then
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
  return 0
}

is_at_least_one_service_running() {
  # Keep this function cohesive with start/stop
  #
  # SUIBASE_DAEMON is an exception to the rule... it should always run!
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

echo_process() {
  local _LABEL=$1
  local _IS_SUPPORTED=$2
  local _PID=$3
  local _INFO=$4

  # Put the label in a 17 character field left aligned and padded with spaces.
  printf "%-17s: " "$_LABEL"

  if ! $_IS_SUPPORTED; then
    echo_blue "DISABLED"
    echo
  else
    if [ -z "$_PID" ]; then
      echo_red "NOT RUNNING"
    else
      echo_blue "OK"
      echo -n " ( pid "
      echo_blue "$_PID"
      echo -n " ) $_INFO"
    fi
    echo
  fi
}
export -f echo_process
