#!/usr/bin/bash

THIS_DIR=$(readlink -f "$(dirname $0)")
BASE_DIR=$(readlink -f "${THIS_DIR}/../../../")
PRODUCT=$(basename ${BASE_DIR})
DIST_DIR="${BASE_DIR}/.github/dist"
cd ${BASE_DIR}

function get_artifacts_versions() {
  VERSION="v$(cat ${BASE_DIR}/Cargo.toml| awk '/^version/{print $3}' | tr "-" "_" | tr -d '"')"
  [ "${VERSION}" == "v0.0.0" ] && unset VERSION
}

function check_artifacts_versions() {
  if [ ! -f ${LIB} ]
  then
    echo "Cannot find $LIB, building it"
    ${THIS_DIR}/build.sh || return 1
  fi

  get_artifacts_versions

  if [ -z "${VERSION}" ]
  then
    echo "Error: Cannot find \$VERSION"
    return 1
  fi
}

function build_venv_j2cli() {
  pushd ${BASE_DIR}/.github >/dev/null

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
  source ${BASE_DIR}/.github/tmp/venv/bin/activate
  j2 $ARGS
  deactivate
}

# BUILD
SRC_DIR="${BASE_DIR}/.github/tmp/${PRODUCT}"
LIB="${DIST_DIR}/lib${PRODUCT}.so"
BUILD_OS_NAME=$(grep ^ID= /etc/os-release | cut -d= -f2 | tr -d '"')
BUILD_OS_NAME=${BUILD_OS_NAME%-leap}
BUILD_OS_VERSION=$(grep ^VERSION_ID= /etc/os-release | cut -d= -f2 | tr -d '"')

check_artifacts_versions || exit 1

mkdir -p ${SRC_DIR}/rpmbuild
cd ${SRC_DIR}/rpmbuild

MAJOR_SLURM_VERSION=$(slurmd --version | cut -d' ' -f2 | awk -F. '{print $1"_"$2}')
PACKAGE_NAME="${PRODUCT}-slurm${MAJOR_SLURM_VERSION}"
RELEASE="0.${BUILD_OS_NAME}.${BUILD_OS_VERSION}"
INPUT_FILE="${SRC_DIR}/input.json"

cat >${INPUT_FILE} <<EOF
{
  "product": "${PACKAGE_NAME}",
  "version": "${VERSION}",
  "release": "${RELEASE}",
  "libdir": "/usr/lib64/slurm",
  "libname": "lib${PRODUCT}.so",
  "distdir": "${DIST_DIR}"
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

j2cli --customize ${CUSTOM_FILE} -f json ${THIS_DIR}/${PRODUCT}.spec.j2 ${INPUT_FILE} > ./${PRODUCT}.spec

ARCH="$(uname -m)"
test -e $ARCH || ln -s . $ARCH
mkdir -p ${PWD}/rpm
rpmbuild --target=$ARCH --clean -ba -D"_topdir ${PWD}/rpm"  ./${PRODUCT}.spec

# INSTALL
OUT_DIR="${DIST_DIR}"
mkdir -p ${OUT_DIR}/src_packages
mv ${SRC_DIR}/rpmbuild/rpm/SRPMS/*.rpm ${OUT_DIR}/src_packages
mkdir -p ${OUT_DIR}/packages
mv ${SRC_DIR}/rpmbuild/rpm/RPMS/${ARCH}/*.rpm ${OUT_DIR}/packages

# CLEAN
rm -rf ${SRC_DIR}
rm -rf ${BASE_DIR}/.github/tmp
rm -f ${LIB}
