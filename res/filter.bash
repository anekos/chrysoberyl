#!/bin/bash
# shellcheck disable=SC2154

set -C
# set -x


function chrysoberyl_filter_main () {
  while read -r line
  do
    [[ $line =~ ^\:\;\ .* ]] || continue;

    if ! eval "$line"
    then
      echo "Error for: $line" 1>&2
      continue
    fi

    case "$event" in
      key)
        if type "key_$name" &> /dev/null && [ -n "$file" ]
        then
          "key_$name" "$file"
        fi
      ;;
      user)
        if [ -n "$key" ]
        then
          "key_$key" "$file"
          echo "key_$key" "$file"
        elif [ -n "$function" ]
        then
          "user_$function" "$file"
          echo "user_$function" "$file"
        fi
      ;;
    esac

    echo "$line"
  done
}
