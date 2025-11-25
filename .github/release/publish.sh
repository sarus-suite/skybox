#!/bin/bash

VERSION_TAG="$1"
THIS_DIR=$(readlink -f $(dirname $0))
BASE_DIR=$(readlink -f "${THIS_DIR}/../../")
DIST_DIR="${THIS_DIR}/dist"
PRODUCT=$(basename ${BASE_DIR})
cd $THIS_DIR

. lib/common.sh
check_build_os || exit 1
check_slurm_version || exit 1
check_build_container_image || exit 1

# PUBLISH
podman run --rm -ti -e SSH_AUTH_SOCK=${SSH_AUTH_SOCK} -v ${SSH_AUTH_SOCK}:${SSH_AUTH_SOCK} -v ${BASE_DIR}:/mnt ${BUILD_IMAGE_NAME} /mnt/.github/release/${BUILD_OS_NAME}/publish_from_container.sh ${VERSION_TAG}
