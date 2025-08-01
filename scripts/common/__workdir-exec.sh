# shellcheck shell=bash

# You must source __globals.sh before __workdir-exec.sh

# workdir_exec() is the key "public" function of this file.

# One command always expected from the user.
CMD_AUTOCOINS_REQ=false
CMD_BUILD_REQ=false
CMD_CREATE_REQ=false
CMD_DELETE_REQ=false
CMD_FAUCET_REQ=false
CMD_LINKS_REQ=false
CMD_PUBLISH_REQ=false
CMD_REGEN_REQ=false
CMD_SET_ACTIVE_REQ=false
CMD_SET_SUI_REPO_REQ=false
CMD_STOP_REQ=false
CMD_START_REQ=false
CMD_STATUS_REQ=false
CMD_UPDATE_REQ=false

usage_local() {
  update_SUIBASE_VERSION_var
  echo_low_green "$WORKDIR"
  echo "  suibase $SUIBASE_VERSION"
  echo
  echo "  Workdir to simulate a Sui network running fully on this machine"
  echo -n "  JSON-RPC API at http://"
  if [ "${CFG_proxy_enabled:?}" != "false" ]; then
    echo "${CFG_proxy_host_ip:?}:${CFG_proxy_port_number:?}"
  else
    echo "localhost:9000"
  fi
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
  echo "    Start $WORKDIR services (will run in background)"
  echo_low_green "   stop"
  echo "     Stop $WORKDIR services"
  echo_low_green "   status"
  echo "   Info about all suibase services."
  echo
  echo_low_green "   create"
  echo "   Create workdir only. This can be useful for changing"
  echo "            the configuration before doing the first start."
  echo
  # 'build' still supported, but suspecting that it is not used
  # by majority of users. So commented out in help for now.
  #echo_low_green "   build"
  #echo "    Download/update local repo and build binaries only. You may"
  #echo "            have to do next an update, start or regen to create a wallet."
  #echo
  echo_low_green "   delete"
  echo "   Delete workdir completely. Can free up a lot of"
  echo "            disk space for when the localnet is not needed."
  echo
  echo_low_green "   update"
  echo "    Update sui/walrus binaries with latest from Mysten Labs"
  echo "             and perform a regen."
  echo
  echo "            Note: When set-sui-repo is configured, there is no git"
  echo "                  operations and this does regen only."
  echo
  echo_low_green "   regen"
  echo "    Build or download binaries, create a wallet as needed and rebuild the"
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

usage_remote() {
  echo_low_green "$WORKDIR"
  echo "  suibase $SUIBASE_VERSION"
  echo
  echo "  Workdir to interact with a remote Sui network"
  if [ "${CFG_proxy_enabled:?}" != "false" ]; then
    echo -n "  JSON-RPC API at http://"
    echo "${CFG_proxy_host_ip:?}:${CFG_proxy_port_number:?}"
  fi
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
  echo "     Start $WORKDIR services (will run in background)"
  echo_low_green "   stop"
  echo "      Stop $WORKDIR services"
  echo_low_green "   status"
  echo "    Info about all suibase services."
  echo
  if [ "$WORKDIR" = "testnet" ] && [ "${CFG_autocoins_enabled:?}" = "true" ]; then
  echo_low_green "   autocoins"
  echo " Manage service to deposit testnet coins to your"
  echo "             account once per day, in exchange for disk space."
  echo
  fi
  echo_low_green "   create"
  echo "    Create workdir only. This can be useful for changing"
  echo "             the configuration before doing the first 'start'."
  echo
  #echo_low_green "   build"
  #echo "     Download/update local repo and build binaries only. You"
  #echo "             may have to do next an 'update' or 'start' to create a wallet."
  #echo
  echo_low_green "   update"
  echo "    Update sui/walrus binaries with latest from Mysten Labs"
  echo "             and create a wallet as needed."
  echo
  echo_low_green "   publish"
  echo "   Publish the module specified in the Move.toml found"
  echo "             in current directory or optional '--path <path>'"
  echo
  echo_low_green "   links"
  echo "     Get RPC server statistics."
  echo
  echo_low_green "   set-active"
  echo
  echo "             Makes $WORKDIR the active context for many"
  echo "             development tools and the 'asui' script."
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
  exit_if_deps_missing

  CMD_REQ=$1
  shift # Consume the command.

  case "$CMD_REQ" in
  start) CMD_START_REQ=true ;;
  stop) CMD_STOP_REQ=true ;;
  status) CMD_STATUS_REQ=true ;;
  create) CMD_CREATE_REQ=true ;;
  build) CMD_BUILD_REQ=true ;;
  delete) CMD_DELETE_REQ=true ;;
  update)
    # shellcheck disable=SC2034 # CMD_UPDATE_REQ not used for now.
    CMD_UPDATE_REQ=true
    ;;
  regen) CMD_REGEN_REQ=true ;;
  publish) CMD_PUBLISH_REQ=true ;;
  links) CMD_LINKS_REQ=true ;;
  set-active) CMD_SET_ACTIVE_REQ=true ;;
  set-sui-repo) CMD_SET_SUI_REPO_REQ=true ;;
  faucet) CMD_FAUCET_REQ=true ;;
  autocoins) CMD_AUTOCOINS_REQ=true ;;
  *) usage ;;
  esac

  # Optional params
  # These are for development/testing/internal and are purposely not
  # docuemnted (not intended for the end-user).
  DEBUG_PARAM=false
  NOBINARY_PARAM=false
  PRECOMPILED_PARAM=false
  NO_AVX2_PARAM=false
  NO_AVX_PARAM=false
  IS_DAEMON_CALL=false
  SHOW_HELP=false

  # This is for the --json parameter
  JSON_PARAM=false

  # Parsing the command line shifting "rule":
  #   -t|--target) target="$2"; shift ;; That's an example with a parameter
  #   -f|--flag) flag=1 ;; That's an example flag

  local AUTOCOINS_SUBCOMMAND=""
  local AUTOCOINS_ADDRESS=""

  case "$CMD_REQ" in
  autocoins)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_PARAM=true ;;
      --daemoncall) IS_DAEMON_CALL=true ;;
      --help) SHOW_HELP=true ;;
      --precompiled | --nobinary)
        error_exit "Option '$1' not compatible with '$CMD_REQ' command"
        ;;
      status|enable|disable|set|purge-data)
        if [ -n "$AUTOCOINS_SUBCOMMAND" ]; then
          error_exit "Only one of 'status', 'enable', 'disable', 'set', or 'purge-data' allowed."
        fi
        AUTOCOINS_SUBCOMMAND="$1"
        if [ "$1" = "set" ] && [ -n "$2" ]; then
          AUTOCOINS_ADDRESS="$2"
          shift  # Consume the address parameter
        fi
        ;;
      *)
        error_exit "Unknown parameter passed: $1"
        ;;
      esac
      shift
    done
    ;; # End parsing autocoins
  faucet)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_PARAM=true ;;
      --daemoncall) IS_DAEMON_CALL=true ;;
      --precompiled | --nobinary)
        echo "Option '$1' not compatible with '$CMD_REQ' command"
        exit 1
        ;;
      *) PASSTHRU_OPTIONS="$PASSTHRU_OPTIONS $1" ;;
      esac
      shift
    done
    ;; # End parsing faucet
  set-sui-repo)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_PARAM=true ;;
      --daemoncall) IS_DAEMON_CALL=true ;;
      --precompiled | --nobinary)
        echo "Option '$1' not compatible with '$CMD_REQ' command"
        exit 1
        ;;
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
    ;; # End parsing set-sui-repo
  publish)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_PARAM=true ;;
      --daemoncall) IS_DAEMON_CALL=true ;;
      --precompiled | --nobinary)
        echo "Option '$1' not compatible with '$CMD_REQ' command"
        exit 1
        ;;
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
      --debug) DEBUG_PARAM=true ;;
      --daemoncall) IS_DAEMON_CALL=true ;;
      --nobinary) NOBINARY_PARAM=true ;; # Will just update the repo.
      --precompiled) PRECOMPILED_PARAM=true ;;
      --no_avx2) NO_AVX2_PARAM=true ;;
      --no_avx)
        NO_AVX2_PARAM=true
        NO_AVX_PARAM=true
        ;;
      *) PASSTHRU_OPTIONS="$PASSTHRU_OPTIONS $1" ;;
      esac
      shift
    done
    ;; # End parsing build
  links)
    while [[ "$#" -gt 0 ]]; do
      case $1 in
      --debug) DEBUG_PARAM=true ;;
      --daemoncall) IS_DAEMON_CALL=true ;;
      --json) JSON_PARAM=true ;;
      --precompiled | --nobinary)
        echo "Option '$1' not compatible with '$CMD_REQ' command"
        exit 1
        ;;
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
      --debug) DEBUG_PARAM=true ;;
      --daemoncall) IS_DAEMON_CALL=true ;;
      --precompiled | --nobinary)
        echo "Option '$1' not compatible with '$CMD_REQ' command"
        exit 1
        ;;
      *)
        echo "Unknown parameter passed: $1"
        exit 1
        ;;
      esac
      shift
    done
    ;; # End parsing default cases
  esac

  # Check for conflicting parameters --precompiled and --nobinary
  if [ "$PRECOMPILED_PARAM" = "true" ] && [ "$NOBINARY_PARAM" = "true" ]; then
    setup_error "Options '--precompiled' and '--nobinary' are conflicting"
  fi

  if [ "$DEBUG_PARAM" = true ]; then
    echo "debug flag set. May run in foreground Ctrl-C to Exit"
  fi

  if [ "$NOBINARY_PARAM" = true ]; then
    echo "nobinary flag set. Will not build sui binaries from the repo."
  fi

  if [ "$PRECOMPILED_PARAM" = true ]; then
    echo "precompiled flag set. Will download binaries from the repo."
  fi

  if [ "$NO_AVX_PARAM" = true ]; then
    echo "no_avx flag set. Will not use AVX and AVX2 instructions."
  else
    if [ "$NO_AVX2_PARAM" = true ]; then
      echo "no_avx2 flag set. Will not use AVX2 instructions."
    fi
  fi

  # Validate further the autocoins subcommands.
  if [ "$CMD_AUTOCOINS_REQ" = true ]; then
    if [ "$WORKDIR" != "testnet" ]; then
      setup_error "Command '$CMD_REQ' not supported for $WORKDIR"
    fi
    case "$AUTOCOINS_SUBCOMMAND" in
    status|enable|disable|purge-data)
      ;;
    set)
      if [ -z "$AUTOCOINS_ADDRESS" ]; then
        setup_error "Sui account address missing after 'set' subcommand"
      fi
      if ! check_is_valid_hex_pk "$AUTOCOINS_ADDRESS"; then
        setup_error "Invalid Sui account address '$AUTOCOINS_ADDRESS'"
      fi
      ;;
    *)
      SHOW_HELP=true
      ;;
    esac
  fi

  # Show subcommand help if requested
  if [ "$SHOW_HELP" = true ]; then
    if [ "$CMD_AUTOCOINS_REQ" = true ]; then
      DISK_SPACE="500MB"
      echo_low_yellow "USAGE: "
      echo
      echo "      $WORKDIR autocoins <SUBCOMMAND>"
      echo
      echo " This service automatically deposit testnet Sui coins daily to"
      echo " your designated address."
      echo
      echo " By default, it 'just works' and you can expect a first deposit"
      echo " within 14 days. Regular daily deposit happens after ~25 days."
      echo
      echo " To keep working normally, a proof-of-installation protocol"
      echo " runs in background and requires:"
      echo "   - the autocoins service to be enabled."
      echo "   - the testnet services to be running (e.g. 'testnet start')"
      echo "   - up to $DISK_SPACE of disk space (you can purge it any time)"
      echo "   - occasional internet access to download+verify disk space."
      echo
      echo " autocoins is design to be hardly noticeable (low CPU and network)"
      echo " except for its disk space requirement to prevent bots abuse."
      echo
      echo_low_yellow "SUBCOMMANDS:"
      echo
      echo_low_green "  status"
      echo "  Shows recent deposit, address etc."
      echo
      echo_low_green "  enable"
      echo "  Enable the autocoins service."
      echo
      echo_low_green "  disable"
      echo " Disable the autocoins service. If re-enabling later it"
      echo "          may take up to 48 hours for deposit to resume. Does not"
      echo "          delete proof-of-installation data (see purge-data)."
      echo
      echo_low_green "  set <address>"
      echo
      echo "        Change deposit address. May take up to 48h to take effect."
      echo
      echo_low_green "  purge-data"
      echo
      echo "        Delete proof-of-installation data (~$DISK_SPACE). If the service"
      echo "        is enabled, then it may take up to 25 days to re-download"
      echo "        this data and having daily deposit normally resume."
      echo
      echo "        Data location:  ~/suibase/workdirs/testnet/autocoins/data"
      exit 0
    fi
  fi

  # Validate if the path exists.
  if [ -n "$OPTIONAL_PATH" ]; then
    if [ ! -d "$OPTIONAL_PATH" ]; then
      echo "Path [ $OPTIONAL_PATH ] not found"
      exit
    fi
  fi

  # CLI Mutex to prevent concurrent conflicting commands.
  # (only "status" is allowed to run concurrently)
  if [ "$CMD_STATUS_REQ" = false ]; then

    cli_mutex_lock "$WORKDIR"
    # Note: Will be unlock on trap EXIT.
  fi

  ###################################################################
  #
  #  Detect if a repair is needed related to the suibase renaming
  #  (this should be removed pass mid-2024 or so...)
  #
  ####################################################################
  # shellcheck disable=SC2153
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
    set-sui-repo|regen)
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

  # shellcheck source=SCRIPTDIR/__apps.sh
  source "$SUIBASE_DIR/scripts/common/__apps.sh"

  # shellcheck source=SCRIPTDIR/__suibase-daemon.sh
  source "$SUIBASE_DIR/scripts/common/__suibase-daemon.sh"
  update_SUIBASE_DAEMON_PID_var

  # shellcheck source=SCRIPTDIR/__dtp-daemon.sh
  source "$SUIBASE_DIR/scripts/common/__dtp-daemon.sh"
  update_DTP_DAEMON_PID_var

  # shellcheck source=SCRIPTDIR/__walrus-binaries.sh
  source "$SUIBASE_DIR/scripts/common/__walrus-binaries.sh"

  if [ "$CMD_AUTOCOINS_REQ" = true ] || [ "$CMD_STATUS_REQ" = true ]; then
    # shellcheck source=SCRIPTDIR/__autocoins.sh
    source "$SUIBASE_DIR/scripts/common/__autocoins.sh"
  fi

  # First, take care of easier read-only commands: "status", "links"

  if [ "$CMD_STATUS_REQ" = true ]; then
    exit_if_workdir_not_ok
    exit_if_sui_binary_not_ok

    local _USER_REQUEST
    _USER_REQUEST=$(get_key_value "$WORKDIR" "user_request")

    update_SUI_VERSION_var

    # Verify from suibase.yaml if proxy services are expected.
    # If yes, then populate STATUS/INFO.
    local _SUPPORT_PROXY
    local _SHOW_PROXY
    local _MLINK_STATUS
    local _MLINK_INFO
    if $IS_DAEMON_CALL; then
      _SUPPORT_PROXY=false
      _SHOW_PROXY=false
    elif [ "${CFG_proxy_enabled:?}" == "false" ]; then
      _SUPPORT_PROXY=false
      _SHOW_PROXY=true
    else
      _SUPPORT_PROXY=true
      _SHOW_PROXY=true
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
        else
          _MLINK_STATUS="DOWN"
          _MLINK_INFO="Proxy not responding"
        fi
        update_JSON_VALUE "info" "$JSON_RESP"
        if [ -n "$JSON_VALUE" ]; then
          _MLINK_INFO="$JSON_VALUE"
        fi
      fi
    fi

    local _SUPPORT_ACOINS
    local _SHOW_ACOINS
    local _ACOINS_STATUS
    local _ACOINS_INFO
    if [ "$WORKDIR" != "testnet" ]; then
      _SUPPORT_ACOINS=false
      _SHOW_ACOINS=false
    elif [ "${CFG_autocoins_enabled:?}" == "false" ]; then
      _SUPPORT_ACOINS=false
      _SHOW_ACOINS=false
    else
      _SUPPORT_ACOINS=true
      _SHOW_ACOINS=true
      autocoins_status "quiet" "$SUIBASE_DAEMON_PID" "$_USER_REQUEST"
      _ACOINS_STATUS=$AUTOCOINS_STATUS
      _ACOINS_INFO=$AUTOCOINS_INFO
    fi

    # Verify from suibase.yaml if sui explorer services are expected.
    # If yes, then populate STATUS/INFO.
    local _SUPPORT_SUI_EXPLORER
    local _SHOW_SUI_EXPLORER
    if $IS_DAEMON_CALL; then
      _SUPPORT_SUI_EXPLORER=false
      _SHOW_SUI_EXPLORER=false
    elif [ "${CFG_sui_explorer_enabled:?}" == "false" ]; then
      _SUPPORT_SUI_EXPLORER=false
      _SHOW_SUI_EXPLORER=$is_local
    else
      _SUPPORT_SUI_EXPLORER=true
      _SHOW_SUI_EXPLORER=$is_local
    fi

    # Verify from suibase.yaml if dtp services are expected.
    # If yes, then populate STATUS/INFO.
    local _SUPPORT_DTP
    local _DTP_STATUS
    local _DTP_INFO
    if [ "${CFG_dtp_enabled:?}" == "false" ]; then
      _SUPPORT_DTP=false
    else
      _SUPPORT_DTP=true
      unset JSON_RESP
      get_dtp_daemon_status "data"
      update_JSON_VALUE "code" "$JSON_RESP"
      if [ -n "$JSON_VALUE" ]; then
        update_JSON_VALUE "message" "$JSON_RESP"
        if [ -n "$JSON_VALUE" ]; then
          _DTP_STATUS="DOWN"
          _DTP_INFO="$JSON_RESP"
        fi
      fi
      if [ -z "$_DTP_STATUS" ]; then
        update_JSON_VALUE "status" "$JSON_RESP"
        if [ -n "$JSON_VALUE" ]; then
          _DTP_STATUS="$JSON_VALUE"
        fi
        update_JSON_VALUE "info" "$JSON_RESP"
        if [ -n "$JSON_VALUE" ]; then
          _DTP_INFO="$JSON_VALUE"
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
          if $_SUPPORT_SUI_EXPLORER && [ -z "$SUIBASE_DAEMON_PID" ]; then
            _DEGRADED=true
          fi
          if $_SUPPORT_DTP && [ -z "$DTP_DAEMON_PID" ]; then
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
        echo_process "Localnet process" true "$SUI_PROCESS_PID"
        echo_process "Faucet process" "$_SUPPORT_FAUCET" "$SUI_FAUCET_PROCESS_PID"
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
      if [ "${CFG_dtp_enabled:?}" = "true" ]; then
        _INFO=$(
          echo -n "http://"
          echo_blue "${CFG_dtp_host_ip:?}"
          echo -n ":"
          echo_blue "${CFG_dtp_web_port_number:?}"
        )
        echo_process "DTP services" "$_SUPPORT_DTP" "$DTP_DAEMON_PID" "$_INFO"
      fi

      if $_SHOW_PROXY; then
        _INFO=$(
          echo -n "http://"
          echo_blue "${CFG_proxy_host_ip:?}"
          echo -n ":"
          echo_blue "${CFG_proxy_port_number:?}"
        )
        echo_process "Proxy server" "$_SUPPORT_PROXY" "$SUIBASE_DAEMON_PID" "$_INFO"
      fi

      if $_SHOW_SUI_EXPLORER; then
        _INFO=$(
          echo -n "http://"
          echo_blue "${CFG_sui_explorer_host_ip:?}"
          echo -n ":"
          echo_blue "${CFG_sui_explorer_port_number:?}"
        )
        echo_process "Explorer" "$_SUPPORT_SUI_EXPLORER" "$SUIBASE_DAEMON_PID" "$_INFO"
      fi

      if [ "$_SUPPORT_PROXY" = true ]; then
        echo -n "Multi-link RPC   : "
        case $_MLINK_STATUS in
        "OK")
          echo_blue "OK"
          ;;
        "DOWN")
          echo_red "DOWN"
          ;;
        esac
        if [ -n "$_MLINK_INFO" ]; then
          echo " ( $_MLINK_INFO )"
        else
          echo
        fi
      fi

      if [ "$_SHOW_ACOINS" = true ]; then
        echo -n "Autocoins        : "
        if [ "$_SUPPORT_ACOINS" = true ]; then
          autocoins_echo_status_color "$_ACOINS_STATUS"
          if [ -n "$_ACOINS_INFO" ]; then
            echo " $_ACOINS_INFO"
          else
            echo
          fi
        else
          echo "DISABLED"
        fi
      fi
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
    show_suibase_daemon_get_links "$DEBUG_PARAM" "$JSON_PARAM"

    exit
  fi

  # A good time to check if the user did mess up with the workdir and fix potentially missing files.
  local _WORKDIR_WAS_OK
  if is_workdir_ok; then
    _WORKDIR_WAS_OK=true
  else
    _WORKDIR_WAS_OK=false
  fi

  repair_workdir_as_needed "$WORKDIR" # Create/repair $WORKDIR

  if [ "$CMD_AUTOCOINS_REQ" = true ]; then
    exit_if_workdir_not_ok

    if [ "$WORKDIR" != "testnet" ]; then
      setup_error "Autocoins is only supported for testnet"
    fi

    if [ "$AUTOCOINS_SUBCOMMAND" = "status" ]; then
      local _USER_REQUEST
      _USER_REQUEST=$(get_key_value "$WORKDIR" "user_request")
      autocoins_status "verbose" "$SUIBASE_DAEMON_PID" "$_USER_REQUEST"
    elif [ "$AUTOCOINS_SUBCOMMAND" = "enable" ]; then
      autocoins_enable "verbose"
    elif [ "$AUTOCOINS_SUBCOMMAND" = "disable" ]; then
      autocoins_disable "verbose"
    elif [ "$AUTOCOINS_SUBCOMMAND" = "set" ]; then
      autocoins_set_address "verbose" "$AUTOCOINS_ADDRESS"
    elif [ "$AUTOCOINS_SUBCOMMAND" = "purge-data" ]; then
      autocoins_purge_data "verbose"
    fi

    exit
  fi

  # Determine how the binary should be produced (build from local repos or downloaded etc...)
  local _WARN_ON_BUILD_FALLBACK_REASON=""

  # Should not download on a regen or set-sui-repo, but still need to do "cargo build" in case the
  # binary are not up to data (or done yet).
  local _ALLOW_REPO_DOWNLOAD="true" # Using string because passing outside as param
  if [ "$CMD_REGEN_REQ" = true ]; then
    _ALLOW_REPO_DOWNLOAD="false"
  fi

  local _IS_SET_SUI_REPO="false"
  if is_sui_repo_dir_override; then
    _ALLOW_REPO_DOWNLOAD="false"
    _IS_SET_SUI_REPO="true"
  fi

  ALLOW_BINARY="true"
  if [ "$NOBINARY_PARAM" = true ]; then
    ALLOW_BINARY="false"
  fi

  DISABLE_AVX="false"
  DISABLE_AVX2="false"
  if [ "$NO_AVX_PARAM" = true ]; then
    DISABLE_AVX="true"
    DISABLE_AVX2="true"
  else
    if [ "$NO_AVX2_PARAM" = true ]; then
      DISABLE_AVX2="true"
    fi
  fi

  local _USE_PRECOMPILED="false"
  if [ "$CMD_BUILD_REQ" = true ]; then
    # Use the --precompiled flag only when 'build' command.
    if [ "$PRECOMPILED_PARAM" = "true" ]; then
      _USE_PRECOMPILED="true"
    fi
  else
    # Use the suibase.yaml with all other commands.
    if [ "${CFG_precompiled_bin:?}" = "true" ]; then
      _USE_PRECOMPILED="true"
    fi
  fi

  # Handle case where precompiled is not compatible.
  if [ "$_USE_PRECOMPILED" = "true" ]; then
    if [ "$_IS_SET_SUI_REPO" = "true" ]; then
      _USE_PRECOMPILED="false"
    else
      update_HOST_vars
      if [ "$HOST_PLATFORM" = "Linux" ]; then
        # Ignore precompiled request if not an Ubuntu x86_64 machine.
        if [ "$HOST_ARCH" != "x86_64" ]; then
          _WARN_ON_BUILD_FALLBACK_REASON="Precompiled binaries not available for '$HOST_ARCH'. Will build from source instead."
          _USE_PRECOMPILED="false"
        fi

        # Disable with WSL/Linux when (AVX or AVX2) not available.
        #
        # For other Linux setup, lets assume the user knows better and allow
        # pre-compiled binaries... (if there is a problem they can work-around this
        # with "precompiled_bin: false" in suibase.yaml)
        #
        # Rational:
        #  A Linux VM guess may NOT report AVX/AVX2 in /proc/cpuinfo, but the host cpu can support it.
        #  The app will still work, because the VM just execute the instructions blindly and they
        #  are not detected as "illegal instructions".
        #  This might be intended by some Linux super-user... but it is less likely intended for WSL
        #  users (more likely the physical host is really not supporting AVX). So protect WSL users
        #  with forcing compilation at the host when no AVX/AVX2 detected.
        if is_wsl; then
          if [[ -f /proc/cpuinfo ]]; then
            local _AVX2_ENABLED
            if grep -q avx2 /proc/cpuinfo; then
              _AVX2_ENABLED="true"
            else
              _AVX2_ENABLED="false"
            fi
            local _AVX_ENABLED
            if grep -q avx /proc/cpuinfo; then
              _AVX_ENABLED="true"
            else
              _AVX_ENABLED="false"
            fi
            if [ "$_AVX2_ENABLED" = "false" ] || [ "$_AVX_ENABLED" = "false" ]; then
              _WARN_ON_BUILD_FALLBACK_REASON="Precompiled binaries not available for WSL/Linux without AVX. Will build from source instead."
              _USE_PRECOMPILED="false"
            fi
          fi
        fi
      else
        if [ "$HOST_PLATFORM" = "Darwin" ]; then
          if [ "$HOST_ARCH" != "x86_64" ] && [ "$HOST_ARCH" != "arm64" ]; then
            _WARN_ON_BUILD_FALLBACK_REASON="Precompiled binaries not available for '$HOST_ARCH'. Will build from source instead."
            _USE_PRECOMPILED="false"
          fi
        else
          # Unsupported OS... "windows" presumably...
          setup_error "Unsupported OS [$HOST_PLATFORM]"
        fi
      fi
    fi

    # Make sure the repo/branch has pre-compiled binary available.
    if [ "$_USE_PRECOMPILED" = "true" ]; then
      if [ "${CFG_default_repo_url:?}" != "${CFGDEFAULT_default_repo_url:?}" ]; then
        # default_repo_url was overriden by the user.
        _WARN_ON_BUILD_FALLBACK_REASON="Precompiled binaries not available for repo '$CFG_default_repo_url'. Will build from source instead."
        _USE_PRECOMPILED="false"
      else
        case "${CFG_default_repo_branch:?}" in
        "devnet" | "testnet" | "mainnet")
          # OK
          ;;
        *)
          _WARN_ON_BUILD_FALLBACK_REASON="Precompiled binaries not available for branch '$CFG_default_repo_branch'. Will build from source instead."
          _USE_PRECOMPILED="false"
          ;;
        esac
      fi
    fi
  fi

  # If precompiled is allowed, then let the config decide if downloading can be avoided.
  if [ "$_USE_PRECOMPILED" = "true" ]; then
    if [ "${CFG_enable_local_repo:?}" = "false" ]; then
      _ALLOW_REPO_DOWNLOAD="false"
    fi
  fi

  # Detect if there was a change to the config that would invalidate
  # the currently installed sui binary.
  #
  # If changes detected, then a call to build_sui_repo_branch (see far below)
  # should be done instead of, say, simply starting the process.
  local _CONFIG_CHANGE_DETECTED
  _CONFIG_CHANGE_DETECTED=false

  # Check if .state/precompiled key exists BUT precompilation is NOT allowed.
  if [ -f "$WORKDIRS/$WORKDIR/.state/precompiled" ] && [ "$_USE_PRECOMPILED" = "false" ]; then
    _CONFIG_CHANGE_DETECTED=true
    # echo "Info: $WORKDIR config change detected. Precompiled binaries not available."
    del_key_value "$WORKDIR" "precompiled"
  fi

  local _REPO_URL_STATE
  _REPO_URL_STATE=$(get_key_value "$WORKDIR" "repo_url")
  local _REPO_URL_EXPECTED
  local _REPO_URL_CHANGE_DETECTED
  _REPO_URL_CHANGE_DETECTED=false
  if [ "$_IS_SET_SUI_REPO" = "true" ]; then
    _REPO_URL_EXPECTED="$RESOLVED_SUI_REPO_DIR"
  else
    _REPO_URL_EXPECTED="${CFG_default_repo_url:?}"
  fi
  if [ "$_REPO_URL_STATE" != "$_REPO_URL_EXPECTED" ]; then
    _CONFIG_CHANGE_DETECTED=true
    _REPO_URL_CHANGE_DETECTED=true
    if [ "$_REPO_URL_STATE" != "NULL" ]; then
      if [ "$_IS_SET_SUI_REPO" = "false" ]; then
        echo "$WORKDIR repo change detected. Repo URL is now [$_REPO_URL_EXPECTED]"
      else
        echo "$WORKDIR repo change detected. set-sui-repo is now [$RESOLVED_SUI_REPO_DIR]"
      fi
      del_key_value "$WORKDIR" "repo_url"
    fi
  fi

  local _REPO_BRANCH_STATE
  _REPO_BRANCH_STATE=$(get_key_value "$WORKDIR" "repo_branch")
  local _REPO_BRANCH_EXPECTED
  if [ "$_IS_SET_SUI_REPO" = "true" ]; then
    _REPO_BRANCH_EXPECTED="$RESOLVED_SUI_REPO_DIR"
  else
    _REPO_BRANCH_EXPECTED="${CFG_default_repo_branch:?}"
  fi
  if [ "$_REPO_BRANCH_STATE" != "$_REPO_BRANCH_EXPECTED" ]; then
    _CONFIG_CHANGE_DETECTED=true
    if [ "$_REPO_BRANCH_STATE" != "NULL" ]; then
      if [ "$_IS_SET_SUI_REPO" = "false" ] && [ "$_REPO_URL_CHANGE_DETECTED" = "false" ]; then
        echo "$WORKDIR config change detected. Branch is [$_REPO_BRANCH_EXPECTED]"
      fi
      del_key_value "$WORKDIR" "repo_branch"
    fi
  fi

  if [ "$CMD_CREATE_REQ" = true ]; then
    # No further action when "create" command.
    if $_WORKDIR_WAS_OK; then
      # Note: did check for repair earlier even if already created (just in case).
      info_exit "$WORKDIR already created"
    else
      info_exit "$WORKDIR created"
    fi
  fi

  # Take care of the case that just stop/start processes.
  if [ "$CMD_START_REQ" = true ]; then

    if is_workdir_ok && is_sui_binary_ok && is_walrus_binary_ok && [ $_CONFIG_CHANGE_DETECTED = false ]; then

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

    if [ "$is_local" == "false" ]; then
      error_exit "faucet command not supported for remote network"
    fi

    # Verify that the faucet is supported for this version.
    if version_less_than "$SUI_VERSION" "sui 0.27"; then
      error_exit "faucet not supported for this older sui version"
    fi

    # Verify that the faucet is enabled and running.
    if [ "${CFG_sui_faucet_enabled:?}" != "true" ]; then
      error_exit "faucet feature disabled (see suibase.yaml )"
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
    start_all_services
    if $is_local && [ -z "$SUI_PROCESS_PID" ]; then
      error_exit "Unable to start $WORKDIR"
    fi
    publish_all "$PASSTHRU_OPTIONS"
    exit
  fi

  if [ "$CMD_SET_ACTIVE_REQ" = true ]; then
    exit_if_workdir_not_ok

    if [ "$ACTIVE_WORKDIR" = "$WORKDIR" ]; then
      info_exit "$WORKDIR is already active"
    else
      set_active_symlink_force "$WORKDIR"
      notify_suibase_daemon_workdir_change
      info_exit "$WORKDIR is now active"
    fi
  fi

  if [ "$CMD_DELETE_REQ" = true ]; then
    stop_all_services

    if ! $is_local; then
      # Do more precise deletion for testnet/devnet/mainnet to preserve key.
      (if cd "$SUI_REPO_DIR" >/dev/null 2>&1; then cargo clean; fi)
      rm -rf "$WORKDIRS/$WORKDIR/logs/sui.log" >/dev/null 2>&1
      rm -rf "$WORKDIRS/$WORKDIR/config/sui-process.log" >/dev/null 2>&1
      rm -rf "$WORKDIRS/$WORKDIR/config/sui-faucet-process.log" >/dev/null 2>&1
      if [ -d "$WORKDIRS/$WORKDIR/sui-repo-default" ]; then
        rm -rf "$WORKDIRS/$WORKDIR/sui-repo-default" >/dev/null 2>&1
        info_exit "Logs, default repo and artifacts deleted. sui.keystore and client.yaml are NEVER deleted for '$WORKDIR'."
      else
        info_exit "$WORKDIR is already deleted"
      fi
    fi

    if [ ! -d "$WORKDIRS/$WORKDIR" ]; then
      info_exit "$WORKDIR is already deleted"
    fi

    echo "Deleting $WORKDIR"

    # TODO Retry a few times until confirmed emptied because of potential race condition
    #      for the $WORKDIRS/$WORKDIR/logs/sui.log (recreated by concurrent status CLI).

    # shellcheck disable=SC2115
    rm -rf "$WORKDIRS/$WORKDIR" >/dev/null 2>&1

    info_exit "$WORKDIR now deleted"
  fi

  # Detect user action that should be NOOP.
  if [ "$CMD_SET_SUI_REPO_REQ" = true ] && [ -z "$OPTIONAL_PATH" ]; then
    if is_sui_repo_dir_default; then
      info_exit "$WORKDIR already using default repo. no change."
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
      rm -rf "$RM_DIR" >/dev/null 2>&1
      rm -rf "$WORKDIRS/$WORKDIR/.state/dns" >/dev/null 2>&1
      rm -rf "$HOME/.sui/sui_config/full_node_db" >/dev/null 2>&1
    fi

    # Delete localnet publish directory (if exists) to force re-publication.
    RM_DIR="$PUBLISHED_DATA_DIR"
    if [ -d "$RM_DIR" ]; then
      rm -rf "$RM_DIR" >/dev/null 2>&1
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
        rm -rf "$RM_DIR" >/dev/null 2>&1
      fi
    fi

    exit
  fi

  # Create and build the sui-repo.

  if [ -n "$_WARN_ON_BUILD_FALLBACK_REASON" ]; then
    warn_user "$_WARN_ON_BUILD_FALLBACK_REASON"
  fi

  build_sui_repo_branch "$_ALLOW_REPO_DOWNLOAD" "$ALLOW_BINARY" "$DISABLE_AVX" "$DISABLE_AVX2" "$_USE_PRECOMPILED" "$PASSTHRU_OPTIONS"

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

  if [ "$ALLOW_BINARY" = false ]; then
    # Can't do anything more than getting the repo.
    return
  fi

  # Check if walrus binaries or configs need to be downloaded/updated.
  update_walrus "$WORKDIR"

  # From this point is the code when a "regen", binaries were potentially
  # updated or creation of the client.yaml/sui.keystore needs to be done.

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

  adjust_sui_aliases "$WORKDIR"

  # Start the local services (will be NOOP if already running).
  start_all_services

  # print sui envs to help debugging (if someone else is using this script).

  CLIENT_YAML_ENVS=$($SUI_EXEC client envs 2>/dev/null | grep -e "$WORKDIR" -e "─" -e "│ active │")
  echo "$CLIENT_YAML_ENVS"

  $SUI_EXEC client addresses 2>/dev/null

  local _ADV="Try it by typing \"$SUI_SCRIPT client gas\""

  update_ACTIVE_ADDRESS_var "$SUI_BIN_DIR/sui" "$CLIENT_CONFIG"
  WALLET_ADDR=$ACTIVE_ADDRESS
  if [ -z "$WALLET_ADDR" ]; then
    echo "There are no addresses in the wallet."
    _ADV="You can create the first wallet address with \"$SUI_SCRIPT client new-address ed25519\""
  else
    COINS_OWNED=$($SUI_EXEC client gas 2>/dev/null)
    # In awk, use $4 field if sui client version >= 1.11.0, otherwise use $3 field.
    update_SUI_VERSION_var
    if version_less_than "$SUI_VERSION" "sui 1.11.0"; then
      COINS_SUM=$(echo "$COINS_OWNED" | awk "{ sum += \$3} END { print sum }")
    else
      COINS_SUM=$(echo "$COINS_OWNED" | awk "{ sum += \$4} END { print sum }")
    fi

    if [ "$COINS_SUM" = "0" ]; then
      echo "Coins owned by $WALLET_ADDR (active): None"
    else
      echo "Coins owned by $WALLET_ADDR (active):"
      echo "$COINS_OWNED"
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
      local _IS_MISMATCH=false
      if [ "$_USE_PRECOMPILED" == false ]; then
        local _BRANCH_NAME
        _BRANCH_NAME=$(if cd "$SUI_REPO_DIR_DEFAULT"; then git branch --show-current; else echo "unknown"; fi)
        if [ "$_BRANCH_NAME" != "$CFG_default_repo_branch" ]; then
          _IS_MISMATCH=true
        fi
      else
        local _DETACHED_INFO
        _DETACHED_INFO=$(if cd "$SUI_REPO_DIR_DEFAULT"; then git branch | grep detached; else echo "unknown"; fi)
        if [[ "$_DETACHED_INFO" != *"$CFG_default_repo_branch"* ]]; then
          _IS_MISMATCH=true
        fi
      fi
      if [ "$_IS_MISMATCH" == true ]; then
        _IS_MISMATCH=true
        warn_user "suibase.yaml is requesting for branch [$CFG_default_repo_branch] but the sui-repo is on [$_BRANCH_NAME]. Do '$WORKDIR update' to fix this."
      fi
    fi
  fi
}

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
