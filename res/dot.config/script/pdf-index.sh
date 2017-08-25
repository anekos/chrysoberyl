#!/bin/bash

set -euC
# set -x

page="$(cat | zenity --list --width 1000 --height 500 --column page --column title)"

printf '@page %q' "$page"
