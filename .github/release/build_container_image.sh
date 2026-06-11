#!/bin/bash

THIS_DIR=$(readlink -f $(dirname $0))
BASE_DIR=$(readlink -f "${THIS_DIR}/../../")
PRODUCT=$(basename ${BASE_DIR})
BUILD_IMAGE_FORCE="yes"
cd $THIS_DIR

. lib/common.sh
check_build_os || exit 1
check_slurm_version || exit 1
check_build_container_image || exit 1
