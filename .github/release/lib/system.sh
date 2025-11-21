#!/usr/bin/bash

function check_os() {
  if [ -f /etc/os-release ]
  then
    OS_NAME=$(cat /etc/os-release | awk -F= '/^ID=/{ print $2}' | tr -d '"')
    OS_VERSION=$(cat /etc/os-release | awk -F= '/^VERSION_ID=/{ print $2}' | tr -d '"')
  fi
  
  case "${OS_NAME}" in
    'sles'|'opensuse-leap')
      OS_NAME="opensuse"
      ;;
  esac

  OS="${OS_NAME}-${OS_VERSION}"
}

check_os
