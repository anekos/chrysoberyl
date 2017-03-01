#!/bin/bash
# shellcheck disable=SC2154

set -eC
# set -x


function chrysoberyl_filter_main () {
  while read -r line
  do
    eval "$line"

    case "$event" in
      key)
        if type "key_$name" &> /dev/null && [ -n "$file" ]
        then
          "key_$name" "$file"
        fi
        ;;
    esac

    echo "$line"
  done
}
