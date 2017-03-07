#!/bin/bash
# shellcheck disable=SC2154

set -C
# set -x


function has_command () {
  type "$1" &> /dev/null
}


function chrysoberyl_filter_call () {
  local line="$1"

  if ! eval "local $line"
  then
    echo "Error for: $line" 1>&2
    return 1
  fi

  case "$event" in
    key)
      if has_command "key_$name" && [ -n "$file" ]
      then
        "key_$name" "$file"
      fi
      ;;
    user)
      if [ -n "$key" ] && has_command "key_$key"
      then
        "key_$key" "$file"
      elif [ -n "$function" ] && has_command "user_$function"
      then
        "user_$function" "$file"
      fi
      ;;
  esac
}


function chrysoberyl_filter_main () {
  while read -r line
  do
    [[ $line =~ ^O=O\ .* ]] && chrysoberyl_filter_call "$line"

    echo "$line"
  done
}
