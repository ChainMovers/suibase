# shellcheck shell=bash

# You must source __globals.sh before __apps.sh

# Install and maintain pre-compiled latest of open-source applications.
#
# When rust sourceable, revert to build locally (as needed).
#
# Apps can optionally be:
#    - installed at "user" or "workdir" granularity.
#    - Run as daemon or CLI.
#    - Binary released indepedently (e.g. walrus, sui...)
#
# By default, the binary will be installed from chainmovers/sui-binaries
#

# All variable members of the app object.

# Arrays to simulate associative arrays
# Can't use "declare" -n" with bash 3.2 (MacOS).
#
# All object are singleton and are access by name like:
#
#  app_call "suibase_daemon" "print"
#  local _BIN_VERSION=$(app_get "suibase_daemon" "2_local_bin_version")
#

# Initialized from defaults/consts.yaml
app_obj_cfg=(
  "assets_name" # Name of the release assets on git.
  "bin_names"   # Comma separated list of binaries to install (from assets_name).
  "install_type" # one binary per user or workdir
  "src_type" # help distinguish how the source code is obtained. suibase|mystenlabs
  "src_path" # info that varies depending of the src_type
  "repo_url"
  "repo_branch"
  "force_tag"
  "asset_name_filter" # Filter for tag names containing this string (use "branch" for branch filtering)
  "build_type"  # For now, supports only "rust"
  "run_type"    # daemon|cli
  "precompiled_bin" # true|false
  "precompiled_type" # suibase|mystenlabs
  "precompiled_path" # path to binaries within the assets.
)

# Initialized with defaults on init_app(), can be modified.
app_obj_vars=(

  "is_initialized" # true|false. Was init_app called on this object?
  "is_installed" # true|false. Are all binaries being installed locally?
  "cache_path" # Path to the cache directory for precompiled binaries.
  "first_bin_name" # First bin name from bin_names (for quick sanity tests).
  "local_bin_path" # Path to the binary installed locally.
  "local_build_path" # Path to the source code while local.

  # Information of the installed binary (from <assets name>-version.yaml).
  "local_bin_version" # The version of the installed binary.
  "local_bin_branch" # The branch of the installed binary (optional).
  "local_bin_commit" # git commit of the installed binary (optional).
  "local_bin_commit_date" # git commit date of the installed binary  (optional).
  "local_bin_origin" # "precompiled" or "built"

   # Information of latest known release (from <assets name>-latest.yaml).
   # Periodically updated by suibase-daemon.
  "local_bin_latest_version"
  "local_bin_latest_branch"
  "local_bin_latest_commit"
  "local_bin_latest_commit_date"
  "local_src_version" # The version of, say, rust toml file (to trig a rebuild when changed).

  # Information retreived remotely.
  "PRECOMP_REMOTE" # true|false Depending if enabled by the user.
  "PRECOMP_REMOTE_PLATFORM" # "ubuntu", "macos" or "windows".
  "PRECOMP_REMOTE_ARCH" # "arm64" or "x86_64"
  "PRECOMP_REMOTE_NOT_SUPPORTED" # "true" if platform/arch not available from precompilation.
  "PRECOMP_REMOTE_VERSION"
  "PRECOMP_REMOTE_TAG_NAME"
  "PRECOMP_REMOTE_DOWNLOAD_URL"
  "PRECOMP_REMOTE_DOWNLOAD_DIR"
  "PRECOMP_REMOTE_FILE_NAME_VERSION"
)

app_obj_params=(
  "cfg_name"
  "workdir"
)

# Public API for the app object.
app_obj_funcs=(
  "print"
  "set_local_vars"
  "install"
  "cleanup_cache"
)

# Function to set a key-value pair
set_app_var() {
  local app_name="$1"
  local key="$2"
  local value="$3"
  local index="${app_name}_${key}"
  eval "${index}='${value}'"
}

# Function to get a value by key without creating a subshell
get_app_var() {
  local app_name="$1"
  local key="$2"
  local index="${app_name}_${key}"
  # shellcheck disable=SC2034
  APP_VAR="${!index}"
}

# Function to call an object's function dynamically
# (virtual function call)
#
# Example:
#   app_call my_app_obj "update"
#   app_call my_app_obj "print
#
app_call() {
  # shellcheck disable=SC2178
  local self=$1
  local func_name=$2
  shift 2

  # TODO Optimize by removing that command call after the scripts "stabilize".

  # Check if the function exists
  get_app_var "$self" "$func_name"
  local func_to_call="${APP_VAR}"
  if [ -z "$func_to_call" ]; then
    setup_error "Error: Function $func_name not found in object $self"
  fi

  $func_to_call "$self" "$@"
}
export -f app_call

init_app_obj() {
  # $1: cfg_name. Application name used in suibase.yaml (e.g. suibase_daemon, walrus)
  # $2: workdir (e.g. "localnet"). Use "" for no workdir.

  # The following are extracted from defaults/consts.yaml
  #
  #   Defines the scope of the installation:
  #      {cfg_name}_install_type: user|workdir
  #
  #   Defines how to get the source code:
  #      {cfg_name}_src_type: ~|suibase|git
  #      {cfg_name}_src_path: ~|git_url|suibase_subdir"
  #
  #   Defines how to build the source code:
  #      {cfg_name}_build_type: ~|rust
  #
  #   Defines how to the binary is executed:
  #      {cfg_name}_run_type: daemon|cli
  #
  #  Defines precompiled binaries support:
  #      {cfg_name}_precompiled_bin: true|false
  #      {cfg_name}_precompiled_type: suibase|mystenlabs
  #      {cfg_name}_precompiled_path: ~|subdir
  #

  local self=$1
  local _WORKDIR=$2

  # Return immediatly if is_initialized exists.
  get_app_var "$self" "is_initialized"
  if [ -n "$APP_VAR" ]; then
    return
  fi

  # Initialized by the user.
  set_app_var "$self" "cfg_name" "$self"
  set_app_var "$self" "workdir" "$_WORKDIR"

  # Variables initialized by defaults/consts.yaml
  for var in "${app_obj_cfg[@]}"; do
    local var_name="CFG_${self}_${var}"
    if [[ -z ${!var_name} ]]; then
      setup_error "Missing variable [$var_name] in defaults/consts.yaml"
    fi
    set_app_var "$self" "$var" "${!var_name:?}"
  done

  # Some variables often read.
  get_app_var "$self" "assets_name"
  local _ASSETS_NAME=$APP_VAR
  get_app_var "$self" "install_type"
  local _INSTALL_TYPE=$APP_VAR

  # Initialize all variable members with a default.
  for var in "${app_obj_vars[@]}"; do
    if [[ $var == is_* ]]; then
      # Initialize differently for likely boolean.
      set_app_var "$self" "$var" "false"
    else
      set_app_var "$self" "$var" ""
    fi
  done

  # Repo info for the sui binary come from suibase.yaml (instead of consts.yaml).
  if [[ $self == "sui" ]]; then
    set_app_var "$self" "repo_url" "${CFG_default_repo_url:?}"
    set_app_var "$self" "repo_branch" "${CFG_default_repo_branch:?}"
    set_app_var "$self" "force_tag" "${CFG_force_tag:?}"
  fi

  # Make sure repo_branch is set to something valid... (because used in some path).
  get_app_var "$self" "repo_branch"
  local _BRANCH=$APP_VAR
  if [[ -z $_BRANCH ]] || [[ $_BRANCH == "~" ]]; then
    _BRANCH="$_WORKDIR"
    set_app_var "$self" "repo_branch" "$_BRANCH"
  fi

  # Set path depending if user vs workdir installation.
  local _local_bin_path
  local _cache_path
  if [[ $_INSTALL_TYPE == "user" ]]; then
    _local_bin_path="suibase/workdirs/common/bin"
    # shellcheck disable=SC2153
    _cache_path="$WORKDIRS/common/.cache/precompiled_downloads/$_ASSETS_NAME/$_BRANCH"
  else
    _local_bin_path="suibase/workdirs/$_WORKDIR/bin"
    _cache_path="$WORKDIRS/$_WORKDIR/.cache/precompiled_downloads/$_ASSETS_NAME/$_BRANCH"
  fi
  set_app_var "$self" "local_bin_path" "$_local_bin_path"
  set_app_var "$self" "cache_path" "$_cache_path"

  # Public virtual functions.
  set_app_var "$self" "set_local_vars" "sb_app_set_local_vars"
  set_app_var "$self" "print" "sb_app_print"
  set_app_var "$self" "install" "sb_app_install"
  set_app_var "$self" "cleanup_cache" "sb_app_cleanup_cache_as_needed"

  # Success.
  set_app_var "$self" "is_initialized" "true"

  # app_call "$self" "print"
}
export -f init_app_obj

sb_app_init_PRECOMP_REMOTE_vars() {
  # Create the "self" reference.
  local self=$1

  set_app_var "$self" "PRECOMP_REMOTE" "false"
  set_app_var "$self" "PRECOMP_REMOTE_PLATFORM" ""
  set_app_var "$self" "PRECOMP_REMOTE_ARCH" ""
  set_app_var "$self" "PRECOMP_REMOTE_NOT_SUPPORTED" ""
  set_app_var "$self" "PRECOMP_REMOTE_VERSION" ""
  set_app_var "$self" "PRECOMP_REMOTE_TAG_NAME" ""
  set_app_var "$self" "PRECOMP_REMOTE_DOWNLOAD_URL" ""
  set_app_var "$self" "PRECOMP_REMOTE_DOWNLOAD_DIR" ""
  set_app_var "$self" "PRECOMP_REMOTE_FILE_NAME_VERSION" ""

  get_app_var "$self" "repo_url"
  local _REPO_URL="$APP_VAR"
  get_app_var "$self" "repo_branch"
  local _BRANCH="$APP_VAR"

  # Make sure _REPO is github (start with "https://github.com")
  if [[ "$_REPO_URL" != "https://github.com"* ]]; then
    warn_user "repo [$_REPO_URL] not supported for pre-compiled binaries"
    return
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
      set_app_var "$self" "PRECOMP_REMOTE_NOT_SUPPORTED" "true"
      return
    fi
  fi

  local _OUT
  local _TAG_NAME

  local _FORCE_TAG_SOURCE
  local _DOWNLOAD_URL
  local _DOWNLOAD_SUBSTRING="$_BIN_PLATFORM-$_BIN_ARCH"

  get_app_var "$self" "force_tag"
  local _FORCE_TAG_NAME=$APP_VAR
  if [ "$_FORCE_TAG_NAME" = "~" ]; then
    _FORCE_TAG_NAME=""
  else
    get_app_var "$self" "cfg_name"
    local _CFG_NAME=$APP_VAR
    if [[ $_CFG_NAME == "sui" ]]; then
      _FORCE_TAG_SOURCE="suibase.yaml"
    else
      _FORCE_TAG_SOURCE="const.yaml"
    fi
    echo "$_FORCE_TAG_SOURCE: Forcing to use tag '[$_FORCE_TAG_NAME]'"
  fi

  update_USER_GITHUB_TOKEN_var

  for _retry_curl in 1 2 3; do
    _DOWNLOAD_URL=""
    _TAG_NAME=""
    if [ -n "$USER_GITHUB_TOKEN" ]; then
      _OUT=$(curl -s --request GET \
        --url "$_REPO_URL/releases" \
        --header "X-GitHub-Api-Version: 2022-11-28" \
        --header "Authorization: Bearer $USER_GITHUB_TOKEN" || true)
    else
      _OUT=$(curl -s --request GET \
        --url "$_REPO_URL/releases" \
        --header "X-GitHub-Api-Version: 2022-11-28" || true)
    fi

    # Extract all tag_name lines from _OUT.
    local _TAG_NAMES
    get_app_var "$self" "install_type"
    local _INSTALL_TYPE=$APP_VAR
    if [[ $_INSTALL_TYPE == "workdir" ]]; then
      # Check asset_name_filter configuration
      get_app_var "$self" "asset_name_filter"
      local _ASSET_NAME_FILTER=$APP_VAR
      if [ -z "$_ASSET_NAME_FILTER" ]; then
        # Empty/null: no filtering beyond tag_name
        _TAG_NAMES=$(echo "$_OUT" | grep "tag_name" | sort_rv)
      elif [ "$_ASSET_NAME_FILTER" = "branch" ]; then
        # "branch": use branch filtering (current behavior)
        _TAG_NAMES=$(echo "$_OUT" | grep "tag_name" | grep "$_BRANCH" | sort_rv)
      else
        # Specific filter: filter tag names containing this string, then extract semantic versions for proper sorting
        # Extract semantic version (x.y.z, x.y, or x) from each tag for version-aware sorting
        _TAG_NAMES=$(echo "$_OUT" | grep "tag_name" | grep "$_ASSET_NAME_FILTER" | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/' | \
          awk '{
            # Extract first x.y.z pattern
            if (match($0, /[0-9]+\.[0-9]+\.[0-9]+/)) {
              version = substr($0, RSTART, RLENGTH)
            }
            # If no x.y.z, extract first x.y pattern
            else if (match($0, /[0-9]+\.[0-9]+/)) {
              version = substr($0, RSTART, RLENGTH)
            }
            # If no x.y, extract first x pattern
            else if (match($0, /[0-9]+/)) {
              version = substr($0, RSTART, RLENGTH)
            }
            else {
              version = $0
            }
            print version "|" $0
          }' | sort_rv | cut -d'|' -f2)
      fi
    else
      _TAG_NAMES=$(echo "$_OUT" | grep "tag_name" | sort_rv)
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
          echo "Warn: Skipping invalid assets $_TAG_NAME"
        fi
      fi
    done <<<"$_TAG_NAMES"

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
        echo "$_FORCE_TAG_SOURCE: tag [$_FORCE_TAG_NAME] not found in remote repo"
        setup_error "Verify force_tag in suibase.yaml is a valid tag for [$_REPO_URL]"
      fi
    else
      setup_error "Could not find a '$_DOWNLOAD_SUBSTRING' binary asset for $_BRANCH in [$_REPO_URL]"
    fi
  fi

  local _TAG_VERSION="${_TAG_NAME#*\-v}" # Remove '-v' and everything before.
  #echo "_OUT=$_OUT"
  #echo "_TAG_NAME=$_TAG_NAME"
  #echo "_TAG_VERSION=$_TAG_VERSION"
  #echo _DOWNLOAD_URL="$_DOWNLOAD_URL"


  # All good. Return success.
  set_app_var "$self" "PRECOMP_REMOTE" "true"
  set_app_var "$self" "PRECOMP_REMOTE_PLATFORM" "$_BIN_PLATFORM"
  set_app_var "$self" "PRECOMP_REMOTE_ARCH" "$_BIN_ARCH"
  set_app_var "$self" "PRECOMP_REMOTE_VERSION" "$_TAG_VERSION"
  set_app_var "$self" "PRECOMP_REMOTE_TAG_NAME" "$_TAG_NAME"
  set_app_var "$self" "PRECOMP_REMOTE_DOWNLOAD_URL" "$_DOWNLOAD_URL"
}
export -f sb_app_init_PRECOMP_REMOTE_vars

sb_app_download_PRECOMP_REMOTE() {

  # Create the "self" reference.
  local self=$1

  get_app_var "$self" "workdir"
  local _WORKDIR=$APP_VAR
  set_app_var "$self" "PRECOMP_REMOTE_DOWNLOAD_DIR" ""
  set_app_var "$self" "PRECOMP_REMOTE_FILE_NAME_VERSION" ""

  get_app_var "$self" "PRECOMP_REMOTE_DOWNLOAD_URL"
  local _PRECOMP_REMOTE_DOWNLOAD_URL=$APP_VAR

  # It is assumed init_PRECOMP_REMOTE_vars was successfully called before
  # and there is indeed something to download and install.
  get_app_var "$self" "PRECOMP_REMOTE"
  local _PRECOMP_REMOTE=$APP_VAR
  if [[ $_PRECOMP_REMOTE != true ]]; then
    return
  fi

  get_app_var "$self" "first_bin_name"
  local _FIRST_BIN_NAME=$APP_VAR

  get_app_var "$self" "local_bin_path"
  local _LOCAL_BIN_PATH=$APP_VAR

  # Download the $_PRECOMP_REMOTE_DOWNLOAD_URL into the cache
  get_app_var "$self" "cache_path"
  local _DOWNLOAD_DIR=$APP_VAR
  mkdir -p "$_DOWNLOAD_DIR"
  local _DOWNLOAD_FILENAME="${_PRECOMP_REMOTE_DOWNLOAD_URL##*/}"
  # Detect and store the file extension type
  local _ARCHIVE_TYPE=""
  local _DOWNLOAD_FILENAME_WITHOUT_EXT=""
  if [[ "$_DOWNLOAD_FILENAME" == *.tgz ]]; then
    _ARCHIVE_TYPE="tgz"
    _DOWNLOAD_FILENAME_WITHOUT_EXT="${_DOWNLOAD_FILENAME%.tgz}"
  elif [[ "$_DOWNLOAD_FILENAME" == *.tar ]]; then
    _ARCHIVE_TYPE="tar"
    _DOWNLOAD_FILENAME_WITHOUT_EXT="${_DOWNLOAD_FILENAME%.tar}"
  else
    setup_error "Unsupported archive format '$_DOWNLOAD_FILENAME'. Expected .tar or .tgz extension."
  fi

  local _DOWNLOAD_FILEPATH="$_DOWNLOAD_DIR/$_DOWNLOAD_FILENAME"
  local _EXTRACT_DIR="$_DOWNLOAD_DIR/$_DOWNLOAD_FILENAME_WITHOUT_EXT" # Where the archive content will be placed.

  local _USE_VERSION=""

  # First location attempted to find the extracted binary.
  local _EXTRACTED_DIR_V1="$_EXTRACT_DIR"
  local _EXTRACTED_TEST_FILENAME_V1=$_FIRST_BIN_NAME
  local _EXTRACTED_TEST_FILEDIR_V1="$_EXTRACTED_DIR_V1/$_EXTRACTED_TEST_FILENAME_V1"

  # Second location attempted.
  local _EXTRACTED_DIR_V2="$_EXTRACT_DIR/$_LOCAL_BIN_PATH"
  local _EXTRACTED_TEST_FILENAME_V2=$_FIRST_BIN_NAME
  local _EXTRACTED_TEST_FILEDIR_V2="$_EXTRACTED_DIR_V2/$_EXTRACTED_TEST_FILENAME_V2"

  # These will be initialized with the version detected in the downloaded file.
  local _EXTRACTED_DIR
  local _EXTRACTED_TEST_FILEDIR

  # TODO validate here the local file is really matching the remote in case of republishing?

  # Try twice before giving up.
  update_USER_GITHUB_TOKEN_var
  get_app_var "$self" "assets_name"
  local _ASSETS_NAME=$APP_VAR
  for i in 1 2; do
    # Download if not already done.
    local _DO_EXTRACTION="false"
    #echo "Checking if $_DOWNLOAD_FILEPATH exists"
    if [ -f "$_DOWNLOAD_FILEPATH" ]; then
      # Check for missing test file.
      if [ ! -f "$_EXTRACTED_TEST_FILEDIR_V1" ] && [ ! -f "$_EXTRACTED_TEST_FILEDIR_V2" ]; then
        _DO_EXTRACTION="true"
      else
        # Check for missing .yaml
        if [ ! -f "$_EXTRACTED_DIR_V1/${_ASSETS_NAME}-version.yaml" ] && [ ! -f "$_EXTRACTED_DIR_V2/${_ASSETS_NAME}-version.yaml" ]; then
          _DO_EXTRACTION="true"
        fi
      fi
    else
      echo "Downloading precompiled $_DOWNLOAD_FILENAME"
      if [ -n "$USER_GITHUB_TOKEN" ]; then
        echo "Using github_token"
        curl -s -L -o "$_DOWNLOAD_FILEPATH" "$_PRECOMP_REMOTE_DOWNLOAD_URL" \
          --header "X-GitHub-Api-Version: 2022-11-28" \
          --header "Authorization: Bearer $USER_GITHUB_TOKEN"
      else
        curl -s -L -o "$_DOWNLOAD_FILEPATH" "$_PRECOMP_REMOTE_DOWNLOAD_URL" \
          --header "X-GitHub-Api-Version: 2022-11-28"
      fi

      # Extract if not already done. This is an indirect validation that the downloaded file is OK.
      # If not OK, delete and try download again.
      _DO_EXTRACTION="true"
    fi

    if [ "$_DO_EXTRACTION" = "true" ]; then
      rm -rf "$_EXTRACT_DIR" >/dev/null 2>&1
      mkdir -p "$_EXTRACT_DIR"
      #echo "Extracting $_DOWNLOAD_FILEPATH into $_EXTRACT_DIR"
      # Use appropriate extraction based on archive type
      if [ "$_ARCHIVE_TYPE" = "tgz" ]; then
        tar -x -z -f "$_DOWNLOAD_FILEPATH" -C "$_EXTRACT_DIR"  # gzipped
      elif [ "$_ARCHIVE_TYPE" = "tar" ]; then
        tar -x -f "$_DOWNLOAD_FILEPATH" -C "$_EXTRACT_DIR"   # uncompressed
      else
        setup_error "Unknown archive type '$_ARCHIVE_TYPE' during extraction"
      fi
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
      # Fallback: search for binary anywhere in archive
      local found_binary
      found_binary=$(find "$_EXTRACT_DIR" -name "$_FIRST_BIN_NAME" -type f 2>/dev/null | head -n 1)
      if [ -n "$found_binary" ]; then
        _USE_VERSION="found"
        _EXTRACTED_DIR=$(dirname "$found_binary")
        _EXTRACTED_TEST_FILEDIR="$found_binary"
      else
        # If extraction is not valid, then delete the downloaded file so it can be tried again.
        _USE_VERSION=""
        if [ $i -lt 2 ]; then
          warn_user "Failed to extract binary. Try again."
          exit 1
        fi
        rm -rf "$_EXTRACT_DIR" >/dev/null 2>&1
        rm -rf "$_DOWNLOAD_FILEPATH" >/dev/null 2>&1
      fi
    fi

    if [ -n "$_USE_VERSION" ]; then

      # Update the version.yaml files for every expected binaries.
      # The output is:
      #   version: self["PRECOMP_REMOTE_VERSION"]
      #   branch: self["local_bin_branch"]
      #   commit: self["local_bin_commit"]
      #   commit-date: self["local_bin_commit_date"]
      #
      # Only the fields that are not empty are written.
      local _VERSION_FILE="$_EXTRACTED_DIR/${_ASSETS_NAME}-version.yaml"
      get_app_var "$self" "PRECOMP_REMOTE_VERSION"
      local _PRECOMP_REMOTE_VERSION=$APP_VAR
      get_app_var "$self" "repo_branch"
      local _REPO_BRANCH=$APP_VAR
      {
        echo "version: \"${_PRECOMP_REMOTE_VERSION}\""
        [ -n "${_REPO_BRANCH}" ] && echo "branch: \"${_REPO_BRANCH}\""
        echo "origin: \"precompiled\""
        #[ -n "${self["local_bin_commit"]}" ] && echo "commit: \"${self["TBD"]}\""
        #[ -n "${self["local_bin_commit_date"]}" ] && echo "commit-date: \"${self["TBD"]}\""
      } >"$_VERSION_FILE"

      # Cleanup cache now that we have likely an older version to get rid of.
      app_call "$self" "cleanup_cache"
      break # Exit the retry loop.
    fi
  done

  # Do a final check that the extracted files are OK.
  if [ ! -f "$_EXTRACTED_TEST_FILEDIR" ]; then
    setup_error "Failed to download or extract precompiled binary for $_BRANCH"
  fi

  # Success
  set_app_var "$self" "PRECOMP_REMOTE_DOWNLOAD_DIR" "$_EXTRACTED_DIR"
  set_app_var "$self" "PRECOMP_REMOTE_FILE_NAME_VERSION" "$_USE_VERSION"
}
export -f sb_app_download_PRECOMP_REMOTE

sb_app_install_PRECOMP_REMOTE() {

  # Create the "self" reference.
  local self=$1

  get_app_var "$self" "workdir"
  local _WORKDIR=$APP_VAR

  get_app_var "$self" "PRECOMP_REMOTE"
  local _PRECOMP_REMOTE=$APP_VAR

  get_app_var "$self" "PRECOMP_REMOTE_DOWNLOAD_DIR"
  local _PRECOMP_REMOTE_DOWNLOAD_DIR=$APP_VAR

  get_app_var "$self" "cfg_name"
  local _CFG_NAME=$APP_VAR

  get_app_var "$self" "local_bin_path"
  local _LOCAL_BIN_PATH=$APP_VAR

  get_app_var "$self" "assets_name"
  local _ASSETS_NAME=$APP_VAR

  local _ALL_INSTALLED=false

  # This assume download_PRECOMP_REMOTE() was successfully completed before.
  if [ "$_PRECOMP_REMOTE" != "true" ] || [ -z "$_PRECOMP_REMOTE_DOWNLOAD_DIR" ]; then
    echo "PRECOMP_REMOTE=$_PRECOMP_REMOTE"
    echo "PRECOMP_REMOTE_DOWNLOAD_DIR=$_PRECOMP_REMOTE_DOWNLOAD_DIR"
    setup_error "Could not install precompiled binary for $_WORKDIR"
  fi

  if [[ $_CFG_NAME == "sui" ]]; then
    # List of Mysten Labs binaries to install.
    local _BINARIES=("sui" "sui-tool" "sui-faucet" "sui-node" "sui-test-validator" "sui-indexer")

    # Detect if a previous build was done, if yes then "cargo clean".
    if [ -d "$SUI_REPO_DIR/target/debug/build" ] || [ -d "$SUI_REPO_DIR/target/release/build" ]; then
      (if cd "$SUI_REPO_DIR"; then cargo clean; else setup_error "Unexpected missing $SUI_REPO_DIR"; fi)
    fi

    # Iterate the BINARIES array and copy/install the binaries.
    # Note: Although the binaries are 'release' we install also
    #       in the debug directory to make it 'easier' to find
    #       for any app.
    local _SRC="$_PRECOMP_REMOTE_DOWNLOAD_DIR"
    for _BIN_NAME in "${_BINARIES[@]}"; do
      local _DST="$WORKDIRS/$_WORKDIR/sui-repo/target/debug"
      # Copy/install files when difference detected.
      sb_app_install_on_bin_diff "$self" "$_SRC" "$_DST" "$_BIN_NAME"
      _DST="$WORKDIRS/$_WORKDIR/sui-repo/target/release"
      sb_app_install_on_bin_diff "$self" "$_SRC" "$_DST" "$_BIN_NAME"
      # This is the new location for workdir binaries (starting 2024).
      _DST="$WORKDIRS/$_WORKDIR/bin"
      sb_app_install_on_bin_diff "$self" "$_SRC" "$_DST" "$_BIN_NAME"
      _ALL_INSTALLED=true
    done
  else
    # Generic installation for most binaries.

    # Build a list of all binaries to be installed.
    get_app_var "$self" "bin_names"
    local _BIN_NAMES_STRING=$APP_VAR
    local OLD_IFS="$IFS"
    IFS=',' read -r -a _BIN_NAMES <<<"${_BIN_NAMES_STRING}"
    IFS="$OLD_IFS"

    local _SRC="$_PRECOMP_REMOTE_DOWNLOAD_DIR"
    local _DST="${HOME}/$_LOCAL_BIN_PATH"
    for _BIN_NAME in "${_BIN_NAMES[@]}"; do
      sb_app_install_on_bin_diff "$self" "$_SRC" "$_DST" "$_BIN_NAME"
    done

    # Install version file.
    sb_app_install_on_bin_diff "$self" "$_SRC" "$_DST" "${_ASSETS_NAME}-version.yaml"

    _ALL_INSTALLED=true
  fi
}
export -f sb_app_install_PRECOMP_REMOTE

sb_app_install_on_bin_diff() {

  # Create the "self" reference.
  local self=$1

  local _SRC="$2"
  local _DST="$3"
  local _BIN="$4"

  # Copy the file _SRC to _DST if the files are binary different.
  # If _DST does not exist, then copy to create it.
  # If _SRC does not exists, then do nothing.
  if [ ! -f "$_SRC/$_BIN" ]; then
    return
  fi
  local _DO_COPY=false
  if [ ! -f "$_DST/$_BIN" ]; then
    _DO_COPY=true
  else
    if ! cmp --silent "$_SRC/$_BIN" "$_DST/$_BIN"; then
      _DO_COPY=true
    fi
  fi
  if [ "$_DO_COPY" = "true" ]; then
    mkdir -p "$_DST"
    \cp -f "$_SRC/$_BIN" "$_DST/$_BIN"

    # Set execute permissions only on expected binaries (from bin_names config)
    get_app_var "$self" "bin_names"
    local _BIN_NAMES_STRING=$APP_VAR
    if [[ ",$_BIN_NAMES_STRING," == *",$_BIN,"* ]]; then
      chmod +x "$_DST/$_BIN"
    fi
  fi
}
export -f sb_app_install_on_bin_diff

sb_app_cleanup_cache_as_needed() {

  # Create the "self" reference.
  local self=$1

  # Do nothing if cache_path is not initialized...
  get_app_var "$self" "cache_path"
  local _CACHE_PATH=$APP_VAR
  if [ -z "$_CACHE_PATH" ]; then
    return
  fi

  # Just cleanup the current cache directory.
  # Only keep last 2 releases for each branch.
  if [ -d "$_CACHE_PATH" ]; then
    # Keep in the cache only the last 2 releases files and latest untar directories (up to 4 items),
    # delete all the rest.
    local _RELEASES
    # shellcheck disable=SC2012 # ls -1 is safe here. find is more risky for portability.
    _RELEASES=$(ls -1 "$_CACHE_PATH" | sort -r)
    local _KEEP=4
    for release in $_RELEASES; do
      if [ -z "$release" ] || [ "$release" = "." ] || [ "$release" = ".." ] || [ "$release" = "/" ]; then
        continue
      fi
      if [ $_KEEP -gt 0 ]; then
        ((_KEEP--))
      else
        # shellcheck disable=SC2115 # $item and $release validated to not be empty string.
        rm -rf "$_CACHE_PATH/$release"
      fi
    done
  fi
}
export -f sb_app_cleanup_cache_as_needed

sb_app_print() {
  # Create the "self" reference.
  # shellcheck disable=SC2178
  local self=$1

  echo "=================="
  echo "$1"
  echo "=================="
  # Display values of all variables in the app object.
  for var in "${app_obj_params[@]}"; do
    # shellcheck disable=SC2128
    get_app_var "$self" "$var"
    echo "  $var: $APP_VAR"
  done
  echo "----"
  for var in "${app_obj_cfg[@]}"; do
    # shellcheck disable=SC2128
    get_app_var "$self" "$var"
    echo "  $var: $APP_VAR"
  done
  echo "----"
  for var in "${app_obj_vars[@]}"; do
    # shellcheck disable=SC2128
    get_app_var "$self" "$var"
    echo "  $var: $APP_VAR"
  done
  echo "----"
  for var in "${app_obj_funcs[@]}"; do
    # shellcheck disable=SC2128
    get_app_var "$self" "$var"
    echo "  $var: $APP_VAR"
  done
  echo "=================="
}


sb_app_set_local_vars() {

  # Set the following variables to quickly reflect what is known
  # locally (no slow network calls allowed here):
  #   local_cached_git_version
  #   local_bin_version
  #   local_src_version

  # Each binary have a <assets_name>-version.yaml file in the bin directory.
  # This is used to detect when a re-installation should be perform.
  #
  # Example of format:
  #   version: "1.2.3"
  #   branch: "main"
  #   commit: "abcdef1234567890"
  #   commit-date: "2022-01-01T12:34:56Z"
  #
  # The version field is mandatory, all others are optional (the more
  # the better to detect changes).
  #
  # These are loaded as BASH variables with a _LOCAL_VER prefix.

  # Create the "self" reference.
  local self=$1

  # Verify if all binaries are installed, create dest dir for binaries (as needed).
  local _ALL_INSTALLED=true
  local _AT_LEAST_ONE_INSTALLED=false

  get_app_var "$self" "local_bin_path"
  local _LOCAL_BIN_PATH=${HOME}/$APP_VAR

  # Iterate the ${self["bin_names"]} string (comma seperated strings) to an array.
  get_app_var "$self" "bin_names"
  local _BIN_NAMES_STRING=$APP_VAR

  get_app_var "$self" "cfg_name"
  local _CFG_NAME=$APP_VAR

  get_app_var "$self" "assets_name"
  local _ASSETS_NAME=$APP_VAR

  local OLD_IFS="$IFS"
  IFS=',' read -r -a _BIN_NAMES <<<"${_BIN_NAMES_STRING}"
  IFS="$OLD_IFS"
  for _BIN_NAME in "${_BIN_NAMES[@]}"; do
    if [ ! "$_AT_LEAST_ONE_INSTALLED" = "true" ]; then
      _AT_LEAST_ONE_INSTALLED=true
      set_app_var "$self" "first_bin_name" "$_BIN_NAME"
    fi
    if [[ ! -f $_LOCAL_BIN_PATH/$_BIN_NAME ]] || [[ ! -x $_LOCAL_BIN_PATH/$_BIN_NAME ]]; then
      _ALL_INSTALLED=false
      break
    fi
  done

  # Load the version.yaml file.
  local _VERSION_FILE="$_LOCAL_BIN_PATH/${_ASSETS_NAME}-version.yaml"
  local _LOCAL_BIN_version=""
  local _LOCAL_BIN_branch=""
  local _LOCAL_BIN_commit=""
  local _LOCAL_BIN_commit_date=""
  local _LOCAL_BIN_origin=""
  if [ -f "$_VERSION_FILE" ]; then
    eval "$(parse_yaml "$_VERSION_FILE" "_LOCAL_BIN_")"
    if [[ -n $_LOCAL_BIN_version ]]; then
      set_app_var "$self" "local_bin_version" "$_LOCAL_BIN_version"
      set_app_var "$self" "local_bin_branch" "$_LOCAL_BIN_branch"
      set_app_var "$self" "local_bin_commit" "$_LOCAL_BIN_commit"
      set_app_var "$self" "local_bin_commit_date" "$_LOCAL_BIN_commit_date"
      set_app_var "$self" "local_bin_origin" "$_LOCAL_BIN_origin"
    fi
    # TODO Handle parsing error.
  else
     # Might be an older installation... so just re-install.
    _ALL_INSTALLED=false
  fi

  # Might get initialized if source code is already available locally.
  local _LOCAL_SRC_VERSION=""

  if [[ $_CFG_NAME == "sui" ]]; then
    # Check in case of the deprecated version.txt file.
    local _SUIBASE_DAEMON_VERSION_INSTALLED=""
    SUIBASE_DAEMON_VERSION_FILE="$SUIBASE_BIN_DIR/$SUIBASE_DAEMON_NAME-version.txt"
    if [[ $_LOCAL_BIN_version == "" ]] && [ -f "$SUIBASE_DAEMON_VERSION_FILE" ]; then
      local _FILE_CONTENT
      _LOCAL_BIN_version=$(cat "$SUIBASE_DAEMON_VERSION_FILE")
      set_app_var "$self" "local_bin_version" "$_LOCAL_BIN_version"
      set_app_var "$self" "local_bin_branch" ""
      set_app_var "$self" "local_bin_commit" ""
      set_app_var "$self" "local_bin_commit_date" ""
      set_app_var "$self" "local_bin_origin" ""
    fi
  else
    # When rust, get the version field from the Cargo.toml
    get_app_var "$self" "build_type"
    local _BUILD_TYPE=$APP_VAR
    if [ "$_BUILD_TYPE" = "rust" ]; then

      local _CARGO_DIR=""
      get_app_var "$self" "src_type"
      local _SRC_TYPE=$APP_VAR
      if [ "$_SRC_TYPE" = "suibase" ]; then
        get_app_var "$self" "src_path"
        local _SRC_PATH=$APP_VAR
        _CARGO_DIR="$SUIBASE_DIR/$_SRC_PATH"
      fi
      if [ -n "$_CARGO_DIR" ]; then
        if [ -f "$_CARGO_DIR/Cargo.toml" ]; then
          # shellcheck disable=SC2086
          _LOCAL_SRC_VERSION=$(grep "^version *= *" $_CARGO_DIR/Cargo.toml | sed -e 's/version[[:space:]]*=[[:space:]]*"\([0-9]\+\.[0-9]\+\.[0-9]\+\)".*/\1/')
          if [ -n "$_LOCAL_SRC_VERSION" ]; then
            set_app_var "$self" "local_src_version" "$_LOCAL_SRC_VERSION"
          fi
        fi
      fi

      # Check if the source code is newer than the local binary (this can happen after "~/suibase/update").
      #
      # Upgrade only if major.minor in source code is higher (ignore patch and build).
      #
      # Only the main branch is considered for upgrade (because the precompiled binary for _LOCAL_SRC_VERSION need
      # to exists).
      if [ "$_ALL_INSTALLED" = "true" ] && [ -n "$_LOCAL_SRC_VERSION" ] && [ -n "$_LOCAL_BIN_version" ] && [ "$_SRC_TYPE" = "suibase" ]; then
        # Get the branch name of the local repo.
        local _LOCAL_BRANCH
        _LOCAL_BRANCH=$(git -C "$SUIBASE_DIR" rev-parse --abbrev-ref HEAD)
        if version_less_than "$_LOCAL_BIN_version" "$_LOCAL_SRC_VERSION"; then
          if [ "$_LOCAL_BRANCH" = "main" ]; then
            _ALL_INSTALLED="false"
            echo "Upgrading $_ASSETS_NAME from $_LOCAL_BIN_version to $_LOCAL_SRC_VERSION"
          else
            echo "Not upgrading $_ASSETS_NAME from $_LOCAL_BIN_version to $_LOCAL_SRC_VERSION (not on main branch: $_LOCAL_BRANCH)"
          fi
        fi
      fi
    fi
  fi

  if [ "$_ALL_INSTALLED" = "true" ]; then
    set_app_var "$self" "is_installed" "true"
  else
    set_app_var "$self" "is_installed" "false"
    mkdir -p "$_LOCAL_BIN_PATH" || setup_error "Failed to create $_LOCAL_BIN_PATH"
  fi

  # Each binary may have a <assets_name>-latest.yaml file in the bin directory.
  # This is used to detect when a new version is available, but not yet installed.
  #
  # Example of format:
  #   version: "1.2.3"
  #   branch: "main"
  #   commit: "abcdef1234567890"
  #   commit-date: "2022-01-01T12:34:56Z"
  #
  # The version field is mandatory, the rest is optional.
  #
  # These are loaded as BASH variables with a _LOCAL_BIN_LATEST prefix.
  local _VERSION_FILE_LATEST="$_LOCAL_BIN_PATH/${_ASSETS_NAME}-latest.yaml"
  if [[ -f $_VERSION_FILE_LATEST ]]; then
    local _LOCAL_BIN_LATEST_version=""
    local _LOCAL_BIN_LATEST_branch=""
    local _LOCAL_BIN_LATEST_commit=""
    local _LOCAL_BIN_LATEST_commit_date=""
    eval "$(parse_yaml "$_VERSION_FILE_LATEST" "_LOCAL_BIN_LATEST")"
    if [[ -n $_LOCAL_BIN_LATEST_version ]]; then
      set_app_var "$self" "local_bin_latest_version" "$_LOCAL_BIN_LATEST_version"
      set_app_var "$self" "local_bin_latest_branch" "$_LOCAL_BIN_LATEST_branch"
      set_app_var "$self" "local_bin_latest_commit" "$_LOCAL_BIN_LATEST_commit"
      set_app_var "$self" "local_bin_latest_commit_date" "$_LOCAL_BIN_LATEST_commit_date"
    fi
  fi

}
export -f sb_app_set_local_vars

sb_app_rust_build_and_install() {
  local self=$1

  get_app_var "$self" "assets_name"
  local _ASSETS_NAME=$APP_VAR

  # Rust (re)build.
  exit_if_rust_build_deps_missing

  # Check if musl target is installed
  local USE_MUSL=false
  if [ "$HOST_PLATFORM" = "Linux" ]; then
    if rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then
      USE_MUSL=true
    else
      echo "Musl target not installed. Using default dynamic compilation"
      echo "To install musl support: rustup target add x86_64-unknown-linux-musl"
    fi
  fi

  # TODO This supports only suibase-daemon for now...

  rm -f "$SUIBASE_DAEMON_VERSION_FILE" >/dev/null 2>&1

  # Clean the build directory.
  rm -rf "$SUIBASE_DAEMON_BUILD_DIR/target" >/dev/null 2>&1

  # Helper function to filter cargo output
  filter_cargo_output() {
    local temp_file
    temp_file="$(mktemp)"
    local exit_code=0

    # Capture all output and exit code
    "$@" > "$temp_file" 2>&1 || exit_code=$?

    # Show errors, warnings, and final status with context
    if [ $exit_code -ne 0 ]; then
      echo "❌ Build failed:"
      # Show errors with more context - include the line before and after
      grep -B1 -A1 -E "(error|Error|ERROR)" "$temp_file" | head -20
      # Also show any compilation failures
      grep -E "(failed|Failed|FAILED)" "$temp_file" | head -5
    else
      # For successful builds, show warnings with file context
      if grep -q -E "(warning|Warning|WARNING)" "$temp_file"; then
        local warnings
        warnings=$(grep -c -E "(warning|Warning|WARNING)" "$temp_file" 2>/dev/null || echo 0)
        echo "⚠️  Build succeeded with $warnings warnings:"
        # Show warning with file context - get the warning and the line that follows (which often has the location)
        grep -A2 -E "warning:" "$temp_file" | head -15
      else
        echo "✅ Build succeeded"
      fi
    fi

    # Save full output to log
    cat "$temp_file" >> "$SUIBASE_LOGS_DIR/cargo-build.log"
    rm -f "$temp_file"

    return $exit_code
  }

  # Build with or without musl
  if [ "$USE_MUSL" = true ]; then
    # Build with musl for static binary
    (if cd "$SUIBASE_DAEMON_BUILD_DIR"; then
      echo "Building $_ASSETS_NAME" with musl target...
      filter_cargo_output cargo build --target x86_64-unknown-linux-musl -p "$SUIBASE_DAEMON_NAME"
     else
      setup_error "unexpected missing $SUIBASE_DAEMON_BUILD_DIR"
     fi)

    # Use the musl build path
    local _SRC="$SUIBASE_DAEMON_BUILD_DIR/target/x86_64-unknown-linux-musl/debug/$SUIBASE_DAEMON_NAME"
  else
    # Standard build
    (if cd "$SUIBASE_DAEMON_BUILD_DIR"; then
      echo "Building $_ASSETS_NAME"...
      filter_cargo_output cargo build -p "$SUIBASE_DAEMON_NAME"
     else
      setup_error "unexpected missing $SUIBASE_DAEMON_BUILD_DIR"
     fi)

    # Use the standard build path
    local _SRC="$SUIBASE_DAEMON_BUILD_DIR/target/debug/$SUIBASE_DAEMON_NAME"
  fi

  # Copy the build result from target to $SUIBASE_BIN_DIR
  if [ ! -f "$_SRC" ]; then
    setup_error "Failed to build $SUIBASE_DAEMON_NAME"
  fi

  # Sanity test that the binary is working.
  local _VERSION
  _VERSION=$("$_SRC" --version)

  # _VERSION should be a string that starts with $SUIBASE_DAEMON_NAME
  if [ -z "$_VERSION" ] || [[ ! "$_VERSION" =~ ^$SUIBASE_DAEMON_NAME ]]; then
    setup_error "Failed to run $SUIBASE_DAEMON_NAME --version"
  fi

  # Remove the leading $SUIBASE_DAEMON_NAME so $_VERSION is just the remaining
  # of the line (with all spaces trimmed).
  _VERSION="${_VERSION#"$SUIBASE_DAEMON_NAME"}"
  _VERSION="${_VERSION#"${_VERSION%%[![:space:]]*}"}"

  # TODO Investigate why this sanity test is failing on MacOS only
  #
  #echo VERSION="$_VERSION"
  #echo SUIBASE_DAEMON_VERSION_SOURCE_CODE="$SUIBASE_DAEMON_VERSION_SOURCE_CODE"
  #if [[ ! "$_VERSION" =~ $SUIBASE_DAEMON_VERSION_SOURCE_CODE$ ]]; then
  #  setup_error "The $SUIBASE_DAEMON_NAME --version ($_VERSION) does not match the expected version ($SUIBASE_DAEMON_VERSION_SOURCE_CODE)"
  #fi

  mkdir -p "$SUIBASE_BIN_DIR"
  \cp -f "$_SRC" "$SUIBASE_DAEMON_BIN"

  # Create the version file.
  local _VERSION_FILE="$SUIBASE_BIN_DIR/${_ASSETS_NAME}-version.yaml"
  get_app_var "$self" "repo_branch"
  local _REPO_BRANCH=$APP_VAR
  {
    echo "version: \"${_VERSION}\""
    [ -n "${_REPO_BRANCH}" ] && echo "branch: \"${_REPO_BRANCH}\""
    echo "origin: \"built\""
    #[ -n "${self["local_bin_commit"]}" ] && echo "commit: \"${self["TBD"]}\""
    #[ -n "${self["local_bin_commit_date"]}" ] && echo "commit-date: \"${self["TBD"]}\""
  } >"$_VERSION_FILE"


  # Clean the build directory.
  rm -rf "$SUIBASE_DAEMON_BUILD_DIR/target" >/dev/null 2>&1
}
export -f sb_app_rust_build_and_install

sb_app_install() {
  # Best-effort attempt to update the app locally.
  # (both binaries and code as needed).
  #
  # Because of potential slow network call, try to call
  # only when an update/installation is needed.
  #
  # Will check for precompiled binaries and fallback
  # to building as needed.

  # It is assumed that init_app_obj() and set_local_vars were already
  # called on self.

  # Create the "self" reference.
  local self=$1

  get_app_var "$self" "assets_name"
  local _ASSETS_NAME=$APP_VAR

  # First check if precompiled binaries is allowed to be done.
  get_app_var "$self" "precompiled_bin"
  local _PRECOMP_ALLOWED=$APP_VAR
  if [ "$_ASSETS_NAME" = "sui" ]; then
    _PRECOMP_ALLOWED=${CFG_precompiled_bin:?}
  fi


  if [ "$_PRECOMP_ALLOWED" = "true" ]; then
    # Attempt a precompiled installation.
    sb_app_init_PRECOMP_REMOTE_vars "$self"
    get_app_var "$self" "PRECOMP_REMOTE_NOT_SUPPORTED"
    local _PRECOMP_REMOTE_NOT_SUPPORTED=$APP_VAR
    # Check if the platform/arch are supported.
    if [ "$_PRECOMP_REMOTE_NOT_SUPPORTED" = "true" ]; then
      get_app_var "$self" "PRECOMP_REMOTE_PLATFORM"
      local _PRECOMP_REMOTE_PLATFORM=$APP_VAR
      get_app_var "$self" "PRECOMP_REMOTE_ARCH"
      local _PRECOMP_REMOTE_ARCH=$APP_VAR
      warn_user "Precompiled binaries not supported for ${_PRECOMP_REMOTE_PLATFORM}-${_PRECOMP_REMOTE_ARCH}"
      _PRECOMP_ALLOWED="false"
    else
      get_app_var "$self" "is_installed"
      local _IS_INSTALLED=$APP_VAR
      local _DO_INSTALL="false"
      if [ "$_IS_INSTALLED" != "true" ]; then
        _DO_INSTALL="true"
      else
        get_app_var "$self" "local_bin_version"
        local _LOCAL_BIN_LATEST_VERSION=$APP_VAR
        get_app_var "$self" "PRECOMP_REMOTE_VERSION"
        local _PRECOMP_REMOTE_VERSION=$APP_VAR
        if [ -n "$_LOCAL_BIN_LATEST_VERSION" ] && [ -n "$_PRECOMP_REMOTE_VERSION" ]; then
          if version_less_than "$_LOCAL_BIN_LATEST_VERSION" "$_PRECOMP_REMOTE_VERSION"; then
            _DO_INSTALL="true"
          fi
        fi
      fi

      if [ "$_DO_INSTALL" = "true" ]; then
        sb_app_download_PRECOMP_REMOTE "$self"
        sb_app_install_PRECOMP_REMOTE "$self"
      fi
    fi
  fi

  if [ "$_PRECOMP_ALLOWED" = "false" ]; then
    # No precompiled allowed, running a dev setup or unsuported platform... so build from source code.
    get_app_var "$self" "build_type"
    local _BUILD_TYPE=$APP_VAR
    if [ "$_BUILD_TYPE" = "rust" ]; then
      sb_app_rust_build_and_install "$self"
    fi
  fi

}
export -f sb_app_install

