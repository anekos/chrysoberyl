#!/bin/bash

set -euC
# set -x


# key b = Set wallpaper
function key_98 () {
  feh --bg-scale "$1"
}

# key y = Copy filepath to clipboard
function key_121 () {
  echo -n "$1" | xclip -i
}

# key p = Push filepath to a file
function key_112 () {
  echo "$1" >> /tmp/chrysoberyl.list
}



IFS=$'\t'

while read -r line
do
  # shellcheck disable=SC2086
  set - $line

  case "$1" in
    Add) continue ;;
    Key)
      if type "key_$2" &> /dev/null
      then
        "key_$2" "$3"
      fi
      ;;
  esac

  echo "$line"
done

