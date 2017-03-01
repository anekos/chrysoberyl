#!/bin/bash

# set -euC
# set -x


source ~/project/chrysoberyl/res/filter.bash


DEST_DIR=/tmp/chrysoberyl


function msg () {
  notify-send -u low Chrysoberyl "$1"
}


# Set wallpaper
function key_b () {
  feh --bg-scale "$1"

  msg Wallpaper
}


# Copy filepath to clipboard
function key_y () {
  echo -n "$1" | xclip -i

  msg Yank
}


# Copy file
function key_p () {
  [ -d "$DEST_DIR" ] || mkdir "$DEST_DIR"

  cp "$1" "$DEST_DIR"

  msg Push
}


# Call main loop
chrysoberyl_filter_main
