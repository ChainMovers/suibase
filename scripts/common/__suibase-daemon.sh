#!/bin/bash

# You must source __globals.sh before __suibase-daemon.sh

SUIBASE_DAEMON_VERSION_FILE="$SUIBASE_BIN_DIR/$SUIBASE_DAEMON_NAME-version.txt"

export SUIBASE_DAEMON_VERSION_INSTALLED=""
update_SUIBASE_DAEMON_VERSION_INSTALLED() {
  # Update the global variable with the version written in the file.
  if [ -f "$SUIBASE_DAEMON_VERSION_FILE" ]; then
    local _FILE_CONTENT
    _FILE_CONTENT=$(cat "$SUIBASE_DAEMON_VERSION_FILE")
    if [ -n "$_FILE_CONTENT" ]; then
      SUIBASE_DAEMON_VERSION_INSTALLED="$_FILE_CONTENT"
      return
    fi
  fi
  SUIBASE_DAEMON_VERSION_INSTALLED=""
}
export -f update_SUIBASE_DAEMON_VERSION_INSTALLED

export SUIBASE_DAEMON_VERSION_SOURCE_CODE=""
update_SUIBASE_DAEMON_VERSION_SOURCE_CODE() {
  # Update the global variable SUIBASE_DAEMON_VERSION_SOURCE_CODE with the version written in the cargo.toml file.
  if [ -f "$SUIBASE_DAEMON_BUILD_DIR/Cargo.toml" ]; then
    local _PARSED_VERSION
    _PARSED_VERSION=$(grep "^version *= *" "$SUIBASE_DAEMON_BUILD_DIR/Cargo.toml" | sed -e 's/version[[:space:]]*=[[:space:]]*"\([0-9]\+\.[0-9]\+\.[0-9]\+\)".*/\1/')
    if [ -n "$_PARSED_VERSION" ]; then
      SUIBASE_DAEMON_VERSION_SOURCE_CODE="$_PARSED_VERSION"
      return
    fi
  fi
  SUIBASE_DAEMON_VERSION_SOURCE_CODE=""
}
export -f update_SUIBASE_DAEMON_VERSION_SOURCE_CODE

need_suibase_daemon_upgrade() {
  # return true if the daemon is not installed, needs to be upgraded
  # or any problem detected.
  #
  # This function assumes that
  #   update_SUIBASE_DAEMON_VERSION_SOURCE_CODE
  #      and
  #   update_SUIBASE_DAEMON_VERSION_INSTALLED
  # have been called before.
  if [ ! -f "$SUIBASE_DAEMON_BIN" ]; then
    true
    return
  fi

  if [ -z "$SUIBASE_DAEMON_VERSION_INSTALLED" ]; then
    true
    return
  fi

  if [ -z "$SUIBASE_DAEMON_VERSION_SOURCE_CODE" ]; then
    true
    return
  fi

  if [ ! "$_SOURCE_VERSION" = "$_INSTALLED_VERSION" ]; then
    true
    return
  fi

  false
  return
}
export -f need_suibase_daemon_upgrade

build_suibase_daemon() {
  #
  # Check to (re)build the daemon as needed.
  # Clean the directory after build. Remain silent if nothing to do.
  #
  # This function assumes that
  #   update_SUIBASE_DAEMON_VERSION_SOURCE_CODE
  #      and
  #   update_SUIBASE_DAEMON_VERSION_INSTALLED
  # have been called before.
  #
  if [ "${CFG_proxy_enabled:?}" != "true" ]; then
    return
  fi

  if need_suibase_daemon_upgrade; then
    echo "Building $SUIBASE_DAEMON_NAME"
    rm -f "$SUIBASE_DAEMON_VERSION_FILE"
    (if cd "$SUIBASE_DAEMON_BUILD_DIR"; then cargo build -p "$SUIBASE_DAEMON_NAME"; else setup_error "unexpected missing $SUIBASE_DAEMON_BUILD_DIR"; fi)
    # Copy the build result from target to $SUIBASE_BIN_DIR
    local _SRC="$SUIBASE_DAEMON_BUILD_DIR/target/debug/$SUIBASE_DAEMON_NAME"
    if [ ! -f "$_SRC" ]; then
      setup_error "Fail to build $SUIBASE_DAEMON_NAME"
    fi
    # TODO Add a sanity test here before overwriting the installed binary.
    mkdir -p "$SUIBASE_BIN_DIR"
    \cp -f "$_SRC" "$SUIBASE_DAEMON_BIN"
    # Update the installed version file.
    echo "$SUIBASE_DAEMON_VERSION_SOURCE_CODE" >|"$SUIBASE_DAEMON_VERSION_FILE"

    # Clean the build directory.
    rm -rf "$SUIBASE_DAEMON_BUILD_DIR/target"
  fi
}
export -f build_suibase_daemon

start_suibase_daemon() {
  # success/failure is reflected by the SUIBASE_DAEMON_PID var.
  # noop if the process is already started.

  if [ "${CFG_proxy_enabled:?}" != "true" ]; then
    return
  fi

  update_SUIBASE_DAEMON_PID_var
  if [ -n "$SUIBASE_DAEMON_PID" ]; then
    return
  fi

  echo "Starting $SUIBASE_DAEMON_NAME"

  # Try until can confirm the suibase-daemon is running healthy, or exit
  # if takes too much time.
  end=$((SECONDS + 30))
  ALIVE=false
  AT_LEAST_ONE_SECOND=false
  for _i in {1..3}; do
    if $SUI_BASE_NET_MOCK; then
      export SUIBASE_DAEMON_PID=$SUI_BASE_NET_MOCK_PID
    else
      # Try to start a script that keeps alive the suibase-daemon.
      #
      # Will not try if there is already another instance running.
      #
      # run-suibase-daemon.sh is design to be flock protected and be silent.
      # All errors will be visible through the suibase-daemon own logs or by observing
      # which PID owns the flock file. So all output of the script (if any) can
      # safely be ignored to /dev/null.
      nohup "$HOME/suibase/scripts/common/run-suibase-daemon.sh" >/dev/null 2>&1 &
    fi

    while [ $SECONDS -lt $end ]; do
      # If CHECK_ALIVE is anything but empty string... then it is alive!
      if $SUI_BASE_NET_MOCK; then
        CHECK_ALIVE="it's alive!"
      else
        # TODO: Actually implement that the assigned port for this workdir is responding!
        CHECK_ALIVE=$(lsof "$SUIBASE_TMP_DIR"/"$SUIBASE_DAEMON_NAME".lock | grep "suibase")
      fi
      if [ -n "$CHECK_ALIVE" ]; then
        ALIVE=true
        break
      else
        echo -n "."
        sleep 1
        AT_LEAST_ONE_SECOND=true
      fi
      # Detect if should do a retry at starting it.
      # This is unlikely needed, but just in case there is a bug in
      # the startup logic...
      if [ $SECONDS -gt $((SECONDS + 10)) ]; then
        break
      fi
    done

    # If it is alive, then break the retry loop.
    if [ "$ALIVE" = true ]; then
      break
    fi
  done

  # Just UI aesthetic newline for when there was "." printed.
  if [ "$AT_LEAST_ONE_SECOND" = true ]; then
    echo
  fi

  # Act on success/failure of the process responding.
  if [ "$ALIVE" = false ]; then
    if [ ! -f "$SUIBASE_DAEMON_BIN" ]; then
      echo "$SUIBASE_DAEMON_NAME binary not found (build failed?)"
    else
      echo "$SUIBASE_DAEMON_NAME not responding. Try again? (may be the host is too slow?)."
    fi
    exit 1
  fi

  update_SUIBASE_DAEMON_PID_var
  echo "$SUIBASE_DAEMON_NAME started (process pid $SUIBASE_DAEMON_PID)"
}
export -f start_suibase_daemon

restart_suibase_daemon() {
  update_SUIBASE_DAEMON_PID_var
  if [ -z "$SUIBASE_DAEMON_PID" ]; then
    start_suibase_daemon
  else
    # This is a clean restart of the daemon (Ctrl-C). Should be fast.
    kill -s SIGTERM "$SUIBASE_DAEMON_PID"
    echo -n "Restarting $SUIBASE_DAEMON_NAME"
    local _OLD_PID="$SUIBASE_DAEMON_PID"
    end=$((SECONDS + 30))
    while [ $SECONDS -lt $end ]; do
      sleep 1
      update_SUIBASE_DAEMON_PID_var
      if [ -n "$SUIBASE_DAEMON_PID" ] && [ "$SUIBASE_DAEMON_PID" != "$_OLD_PID" ]; then
        echo " (new pid $SUIBASE_DAEMON_PID)"
        return
      fi
    done
    echo " (failed for pid $SUIBASE_DAEMON_PID)"
  fi
}
export -f restart_suibase_daemon

stop_suibase_daemon() {
  # success/failure is reflected by the SUI_PROCESS_PID var.
  # noop if the process is already stopped.
  update_SUIBASE_DAEMON_PID_var
  if [ -n "$SUIBASE_DAEMON_PID" ]; then
    echo "Stopping $SUIBASE_DAEMON_NAME (process pid $SUIBASE_DAEMON_PID)"

    if $SUI_BASE_NET_MOCK; then
      unset SUIBASE_DAEMON_PID
    else
      # TODO This will just restart the daemon... need to actually kill the parents as well!
      kill -s SIGTERM "$SUIBASE_DAEMON_PID"
    fi

    # Make sure it is dead.
    end=$((SECONDS + 15))
    AT_LEAST_ONE_SECOND=false
    while [ $SECONDS -lt $end ]; do
      update_SUIBASE_DAEMON__PID_var
      if [ -z "$SUIBASE_DAEMON__PID" ]; then
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

    if [ -n "$SUIBASE_DAEMON__PID" ]; then
      setup_error " $SUIBASE_DAEMON_NAME pid=$SUIBASE_DAEMON__PID still running. Try again, or stop (kill) the process yourself before proceeding."
    fi
  fi
}
export -f stop_suibase_daemon

export SUIBASE_DAEMON_VERSION=""
update_SUIBASE_DAEMON_VERSION_var() {
  # This will be to get the version from the *running* daemon.
  # --version not supported yet. Always mock it for now.
  SUIBASE_DAEMON_VERSION="0.0.0"
}
export -f update_SUIBASE_DAEMON_VERSION_var

export SUIBASE_DAEMON_PID=""
update_SUIBASE_DAEMON_PID_var() {
  if $SUI_BASE_NET_MOCK; then return; fi

  local _PID
  _PID=$(lsof -t -a -c suibase "$SUIBASE_TMP_DIR"/"$SUIBASE_DAEMON_NAME".lock)

  #shellcheck disable=SC2181
  if [ $? -eq 0 ]; then
    # Verify _PID is a number. Otherwise, assume it is an error.
    if [[ "$_PID" =~ ^[0-9]+$ ]]; then
      export SUIBASE_DAEMON_PID="$_PID"
      return
    fi
  fi
  # For all error case.
  SUIBASE_DAEMON_PID=""
}
export -f update_SUIBASE_DAEMON_PID_var
