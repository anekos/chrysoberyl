#!/bin/bash

set -euC
# set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app

function dialog {
  yanity --file-selection \
    --multiple \
    --directory \
    --separator="\n"
}

for it in $(dialog)
do
  printf '@push-directory %q\n' "$it"
done
