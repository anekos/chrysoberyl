#!/bin/bash

set -euC
# set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app


op="$(yanity --entry --width 500 --entry-text="${CHRY_X_LAST_SHELL:-}")"

printf '@set-env CHRY_X_LAST_SHELL %q\n' "$op"
echo "$op"


