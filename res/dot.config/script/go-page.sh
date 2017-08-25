#!/bin/bash

set -euC
set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app

# shellcheck disable=SC2012
yanity --modal --attach "$WINDOWID" --scale --print-partial --text "$CHRY_PAGES pages" --value "$CHRY_PAGE" --min-value 1 --max-value "$CHRY_PAGES" | while read -r page
do
  printf '@first %q\n' "$page"
done

