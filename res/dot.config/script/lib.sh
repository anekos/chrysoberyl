
function has_command {
  which "$1" > /dev/null 2>&1
}

function message {
  if has_command gxmessage
  then
    gxmessage "$@"
  elif has_command xmessage
  then
    xmessage "$@"
  else
    echo "$@" 2>&1
  fi
}

function check_app {
  if has_command yad || has_command zenity
  then
    :
  else
    message "Please install zenity or yad"
    exit 1
  fi
}

function yanity {
  if has_command yad
  then
    yad "$@"
  else
    zenity "$@"
  fi
}
