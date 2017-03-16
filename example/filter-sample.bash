#!/bin/bash

set -C
# set -x


function msg () {
  notify-send -u low Chrysoberyl "$1"
}


function chrysoberyl_filter_call () {
  local line="$1"

  if ! eval "local $line"
  then
    echo "Error for: $line" 1>&2
    return 1
  fi

  # shellcheck disable=SC2154
  if [ "$event" = HTTP ] && [ "$state" = "done" ] && [ "$queue" = 0 ]
  then
    msg 'Queue Empty'
  fi
}


while read -r line
do
  [[ $line =~ ^O=O\ .* ]] && chrysoberyl_filter_call "$line"

  echo "$line"
done
