#!/bin/bash
# shellcheck disable=SC2154

set -C
# set -x


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
    notify-send Chrysoberyl 'HTTP: Done'
  fi
}


function chrysoberyl_filter_main () {
  while read -r line
  do
    [[ $line =~ ^O=O\ .* ]] && chrysoberyl_filter_call "$line"

    echo "$line"
  done
}
