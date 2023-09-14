#!/bin/bash

WORKDIR="localnet"

_ARGS=("$WORKDIR" "$@")

# shellcheck source=SCRIPTDIR/../../tests/020_workdir_exec/__test_common.sh
source "$HOME/suibase/scripts/tests/020_workdir_exec/__test_common.sh" "${_ARGS[@]}"
