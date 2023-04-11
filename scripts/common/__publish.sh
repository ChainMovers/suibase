#!/bin/bash

# Code that does publish modules to a Sui network

# Intended to be sourced only in __workdir-exec.sh

publish_local() {

  local _PASSTHRU_OPTIONS="$@"

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
  publish_clear_output "$INSTALL_DIR";

  # Run unit tests.
  #script_cmd "lsui move test --install-dir \"$INSTALL_DIR\" -p \"$MOVE_TOML_DIR\""

  # Build the Move package for publication.
  #echo Now publishing on network
  CMD="$SUI_EXEC client publish --gas-budget 400000000 --install-dir \"$INSTALL_DIR\" \"$MOVE_TOML_DIR\" $_PASSTHRU_OPTIONS --json 2>&1 1>$INSTALL_DIR/publish-output.json"

  echo $CMD
  echo "sui-base: Publishing..."
  script_cmd $CMD;

  #  TODO Investigate problem with exit status here...

  # Create the created_objects.json file.
  echo -n "[" > "$INSTALL_DIR/created-objects.json";
  local _first_object_created=true
  # Get all the objectid
  awk '/"created":/,/],/' "$INSTALL_DIR/publish-output.json" |
  grep objectId | sed 's/\"//g; s/,//g' | tr -d "[:blank:]" |
  while read -r line ; do
    # Extract first hexadecimal literal found.
    # Define the seperator (IFS) as the JSON ':'
    local _ID=""
    IFS=":"
    for _i in $line
    do
      if beginswith 0x "$_i"; then
        _ID=$_i
        break;
      fi
    done
    # Best-practice to revert IFS to default.
    unset IFS
    #echo "$_ID"
    if [ -n "$_ID" ]; then
      # Get the type of the object
      object_type=$($SUI_EXEC client object "$_ID" --json | grep "type" | sed 's/,//g' | tr -d "[:blank:]" | head -n 1)
      if [ -z "$object_type" ]; then
        # To be removed eventually. Version 0.27 devnet was working differently.
        object_type=$($SUI_EXEC client object "$_ID" --json | grep "dataType" | grep "package")
        if [ -n "$object_type" ]; then
          _found_id=true
        fi
      else
        if $_first_object_created; then
          _first_object_created=false
        else
          echo "," >> "$INSTALL_DIR/created-objects.json";
        fi

        echo -n "{\"objectid\":\"$_ID\",$object_type}" >> "$INSTALL_DIR/created-objects.json";
        #echo "ot=[$object_type]"
        if [ "$object_type" = "\"type\":\"package\"" ]; then
          _found_id=true
        fi
      fi

      if $_found_id; then
        JSON_STR="[\"$_ID\"]"
        echo "$JSON_STR" > "$INSTALL_DIR/package-id.json"
        _found_id=false
      fi

    fi
  done
  echo "]" >> "$INSTALL_DIR/created-objects.json";

  # Load back the package-id.json from the file for validation
  _ID_PACKAGE=$(sed 's/\[//g; s/\]//g; s/"//g;' "$INSTALL_DIR/package-id.json")

  # echo "Package ID=[$_ID_PACKAGE]"

  if [ -z "$_ID_PACKAGE" ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "sui-base: Publication failed."
  fi

  # Test the publication by retreiving object information from the network
  # using that parsed package id.
  script_cmd "$SUI_EXEC client object $_ID_PACKAGE"
  echo "sui-base: Verifying new package is on network..."
  validation=$($SUI_EXEC client object "$_ID_PACKAGE" | grep -i "package")
  if [ -z "$validation" ]; then
    cat "$INSTALL_DIR/publish-output.json"
    setup_error "sui-base: Unexpected object type (Not a package)"
  fi
  JSON_STR="[\"$_ID_PACKAGE\"]"
  echo "$JSON_STR" > "$INSTALL_DIR/package-id.json"

  echo "Package ID is $JSON_STR"
  echo "Also written in [$INSTALL_DIR/package-id.json]"
  echo "Publication Successful"
}
export -f publish_local
