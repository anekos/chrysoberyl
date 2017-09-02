#!/bin/bash

set -euC
# set -x

# shellcheck disable=SC1090
. "$(dirname "$0")/lib.sh"

check_app

function dialog {
  IMAGES='*.png *.gif *.jpg *.jpeg *.bmp'
  ARCHIVES='*.zip *.lha *.rar *.lzh *.pdf'

  zenity --file-selection \
    --multiple \
    --separator="\n" \
    --file-filter "Supported files | $ARCHIVES $IMAGES" \
    --file-filter "PDF | *.pdf" \
    --file-filter "Archive | $ARCHIVES" \
    --file-filter "Image | *$IMAGES" \
    --file-filter "All files (*.*)"
}

for it in $(dialog)
do
  printf '@push %q\n' "$it"
done
