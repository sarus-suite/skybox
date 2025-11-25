#!/bin/bash

THIS_DIR=$(readlink -f $(dirname $0))
BASE_DIR=$(readlink -f "${THIS_DIR}/../../")
DIST_DIR="${THIS_DIR}/dist"
PRODUCT=$(basename ${BASE_DIR})
LIB="${DIST_DIR}/lib${PRODUCT}.so"
cd $THIS_DIR

echo "ENVIRONMENT"
env
echo "OS"
cat /etc/os-release | grep PRETTY
