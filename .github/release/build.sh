#!/bin/bash

THIS_DIR=$(readlink -f $(dirname $0))
BASE_DIR=$(readlink -f "${THIS_DIR}/../../")
DIST_DIR="${THIS_DIR}/dist"
PRODUCT=$(basename ${BASE_DIR})
LIB="${DIST_DIR}/lib${PRODUCT}.so"
cd $THIS_DIR

. lib/common.sh
check_build_os || exit 1
check_slurm_version || exit 1
check_build_container_image || exit 1

# CLEAN old artifact
rm -rf ${LIB}

# BUILD
podman run --rm -ti -v ${BASE_DIR}:/tmp ${BUILD_IMAGE_NAME} /tmp/.github/release/${BUILD_OS_NAME}/build_in_container.sh

# CLEAN temporary files
rm -rf ${THIS_DIR}/tmp
