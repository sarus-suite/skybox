#!/bin/bash
# 
# Fetch the binary, just build a rpm.
#

THIS_DIR=$(readlink -f $(dirname $0))
BASE_DIR=$(readlink -f "${THIS_DIR}/../../")
DIST_DIR="${THIS_DIR}/dist"
PRODUCT=$(basename ${BASE_DIR})
cd $THIS_DIR

. lib/common.sh
check_build_os || exit 1
check_slurm_version || exit 1
check_build_container_image || exit 1

# BUILD
SRC_DIR="${THIS_DIR}/tmp/${PRODUCT}"
LIB="${DIST_DIR}/lib${PRODUCT}.so"

mkdir -p ${SRC_DIR}
cd ${SRC_DIR}

function get_artifacts_versions() {
  unset VERSION
  VERSION="v$(cat ${BASE_DIR}/Cargo.toml| awk '/^version/{print $3}' | tr "-" "_" | tr -d '"')"
}

function check_artifacts_versions() {
  if [ -z "${VERSION}" ]
  then
    echo "Error: Cannot find \$VERSION"
    return 1
  fi
  if [ ! -f ${LIB} ]
  then
    echo "Cannot find $LIB, building it"
    ${THIS_DIR}/build.sh || return 1
  fi	  
}

get_artifacts_versions
check_artifacts_versions || exit 1

mkdir -p ${SRC_DIR}/rpmbuild
cd ${SRC_DIR}/rpmbuild

MAJOR_SLURM_VERSION=$(echo $SLURM_VERSION | awk -F. '{print $1"_"$2}')
PACKAGE_NAME="${PRODUCT}-slurm${MAJOR_SLURM_VERSION}"
RELEASE="0.${BUILD_OS_NAME}.${BUILD_OS_VERSION}"
INPUT_FILE="${SRC_DIR}/input.json"

cat >${INPUT_FILE} <<EOF
{
  "product": "${PACKAGE_NAME}",
  "version": "${VERSION}",
  "release": "${RELEASE}",
  "libdir": "/usr/lib64/slurm",
  "libname": "lib${PRODUCT}.so" 
}
EOF

CUSTOM_FILE="${SRC_DIR}/rpmbuild/custom.py"
cat >${CUSTOM_FILE} <<EOF
def j2_environment_params():
    return dict(
        # Remove whitespace around blocks
        trim_blocks=True,
        lstrip_blocks=True
    )
EOF

j2cli --customize ${CUSTOM_FILE} -f json ${THIS_DIR}/${BUILD_OS_NAME}/${PRODUCT}.spec.j2 ${INPUT_FILE} > ./${PRODUCT}.spec

cp ${THIS_DIR}/${BUILD_OS_NAME}/build_rpm_in_container.sh ./build_rpm_in_container.sh
cp ${LIB} ./

podman run --rm -ti -e PRODUCT=${PRODUCT} -v ${SRC_DIR}/rpmbuild:/tmp ${BUILD_IMAGE_NAME} /tmp/build_rpm_in_container.sh

# INSTALL
OUT_DIR="${DIST_DIR}"
mkdir -p ${OUT_DIR}/SRPMS
mv ${SRC_DIR}/rpmbuild/rpm/SRPMS/*.rpm ${OUT_DIR}/SRPMS/
mkdir -p ${OUT_DIR}/RPMS/${ARCH}
mv ${SRC_DIR}/rpmbuild/rpm/RPMS/${ARCH}/*.rpm ${OUT_DIR}/RPMS/${ARCH}/

# CLEAN
rm -rf ${SRC_DIR}
rm -rf ${THIS_DIR}/tmp
