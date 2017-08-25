#!/bin/bash

set -euC
# set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app

zenity --entry


