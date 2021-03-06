#!/bin/bash

set -euC
# set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app

DIR="$(dirname "${CHRY_PATH:-$PWD}")"
NAME="${CHRY_BASE_NAME:-export.png}"

function dialog {
  yanity --file-selection \
    --filename="$DIR/$NAME" \
    --confirm-overwrite \
    --save \
    --file-filter "All files (*.*)"
}


path="$(dialog)"

printf '@file copy %q %q\n' "$(dirname "$path")" "$(basename "$path")"
