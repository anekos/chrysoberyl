#!/bin/bash

set -euC
# set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app

DIR=~/.cache/chrysoberyl/session

function dialog {
  zenity --file-selection --multiple \
    --filename="$DIR/session.chry" \
    --file-filter "Chrysoberyl | *.chry" \
    --file-filter "All files (*.*)"
}

path="$(dialog)"

printf '@load %q\n' "$path"
