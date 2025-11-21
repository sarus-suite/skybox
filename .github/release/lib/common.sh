#!/bin/bash

function get_github_repo_latest_release() {
  local REPO="${1}"
  local URL="https://api.github.com/repos/${REPO}/releases/latest"
  ( curl -s ${URL} | jq -r ".tag_name" )
  return $?
}

function build_venv_j2cli() {
  pushd $THIS_DIR >/dev/null

  if [ ! -f tmp/venv/bin/activate ]
  then
    # jinja2-cli
    mkdir -p ./tmp
    python3 -m venv tmp/venv
    source tmp/venv/bin/activate
    python3 -m pip install --upgrade pip &>/dev/null
    pip install j2cli &>/dev/null
  fi

  popd >/dev/null
}

function j2cli() {
  ARGS=$@	
  build_venv_j2cli
  source ${THIS_DIR}/tmp/venv/bin/activate
  j2 $ARGS
  deactivate
}

function create_tmp_folders() {
  TMP_DIR="${SCRIPT_DIR}/tmp/${BUILD_OS}/${ARCH}"
  [ ! -d "${TMP_DIR}" ] && mkdir -p ${TMP_DIR}
  BUILD_DIR="${TMP_DIR}/build"
  [ ! -d "${BUILD_DIR}" ] && mkdir -p ${BUILD_DIR}

  if [ "${USERSPACE_RUN}" == 'yes' ]
  then
    USERSPACE_DIR="$(readlink -f $(dirname ${BASE_DIR}))"
  else	  
    USERSPACE_DIR="${TMP_DIR}/userspace"
  fi
  [ ! -d "${USERSPACE_DIR}" ] && mkdir -p ${USERSPACE_DIR}

  PACKAGES_DIR="${TMP_DIR}/packages"
  [ ! -d "${PACKAGES_DIR}" ] && mkdir -p ${PACKAGES_DIR}
  ARTIFACTS_DIR="${TMP_DIR}/artifacts"
  [ ! -d "${ARTIFACTS_DIR}" ] && mkdir -p ${ARTIFACTS_DIR}
}

function get_github_org() {
  local PRODUCT=$1
  local GITHUB_ORG=""
  case "$PRODUCT" in
    parallax|sarusctl)
      GITHUB_ORG="sarus-suite"
      ;;      
    conmon|crun|fuse-overlayfs|podman)
      GITHUB_ORG="containers"
      ;;      
    *)
      echo "NOT_FOUND"
      echo "ERROR: Cannot find GITHUB organization for ${PRODUCT}" >&2
      return 1
      ;;      
  esac

  echo "${GITHUB_ORG}"
  return 0  
}

function github_fetch() {

  local PRODUCT="$1"
  local RELEASE="$2"
  local ARTIFACT="$3"

  local BASE_URL="https://github.com"
  local GITHUB_ORG=$(get_github_org ${PRODUCT}) || return 1
  
  URL="${BASE_URL}/${GITHUB_ORG}/${PRODUCT}/releases/download/${RELEASE}/${ARTIFACT}"

  curl -sOL ${URL}
  RC=$?
  if [ $RC -ne 0 ] || [ ! -f "${ARTIFACT}" ]
  then
    echo "ERROR: Cannot download ${URL}"
    return 1
  else
    if ( file -b ${ARTIFACT} | grep -q "^ASCII" )
    then
      if [ "$(cat ${ARTIFACT})" == "Not Found" ]
      then
        rm -f ${ARTIFACT}
        echo "ERROR: Cannot find artifact at ${URL}"
	return 1
      fi	      
    fi	    
  fi	  
  return 0
}

function github_fetch_sources() {
  local PRODUCT="$1"

  local BASE_URL="https://github.com"
  local GITHUB_ORG=$(get_github_org ${PRODUCT}) || return 1
  local GIT_REPO_URL="${BASE_URL}/${GITHUB_ORG}/${PRODUCT}.git"

  # clean-up previous
  rm -rf ${PRODUCT}

  if [ -n "$GIT_TAG" ]
  then
    local GIT_BRANCH_OPT="--branch ${GIT_TAG} --depth 1"
  elif [ -n "$GIT_BRANCH" ]
  then
    local GIT_BRANCH_OPT="--branch ${GIT_BRANCH} --depth 1"
  else
    local GIT_BRANCH_OPT=""
  fi

  git clone ${GIT_BRANCH_OPT} ${GIT_REPO_URL} ${PRODUCT}
  pushd ${PRODUCT} >/dev/null

  if [ -n "$GIT_COMMIT" ]
  then
    git checkout ${GIT_COMMIT}
  fi

  popd >/dev/null
}

if [ -z "${ARCH}" ] || [ -z "${GOARCH}" ]
then	
  ARCH=$(uname -m)

  if [ "$ARCH" == "x86_64" ]
  then
    GOARCH="amd64"
  else
    GOARCH="$ARCH"
  fi
fi

. lib/system.sh
. lib/build_os.sh
. lib/slurm_version.sh
. lib/build_container_image.sh
