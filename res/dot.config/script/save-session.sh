#!/bin/bash

set -euC
# set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app

DIR=~/.cache/chrysoberyl/session

mkdir -p "$DIR"

function dialog {
  yanity --file-selection \
    --multiple \
    --filename="$DIR/session.chry" \
    --confirm-overwrite \
    --save \
    --file-filter "Chrysoberyl | *.chry" \
    --file-filter "All files (*.*)"
}

path="$(dialog)"

printf '@save %q\n' "$path"
