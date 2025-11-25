#!/bin/bash

function list_packages() {

  for file in $(ls ${BASE_DIR}/.github/release/${BUILD_OS_NAME}/*.packages 2>/dev/null)
  do
    cat ${file}
  done | sort -u | paste -s -d " " 

}

function add_containerfile_excerpts() {

  for file in $(ls ${BASE_DIR}/.github/release/${BUILD_OS_NAME}/*.Containerfile 2>/dev/null)
  do
    cat ${file}
  done
  ARCH=$(uname -m)
  for file in $(ls ${BASE_DIR}/.github/release/${BUILD_OS_NAME}/*.Containerfile.${ARCH} 2>/dev/null)
  do
    cat ${file}
  done
}

function build_container_image_opensuse() {
  
  SRC_DIR="tmp/${BUILD_OS}/${ARCH}/build/container_image"
  [ ! -d ${SRC_DIR} ] && mkdir -p ${SRC_DIR}
  
  BASE_IMAGE_NAME="${BUILD_OS_NAME}/leap:${BUILD_OS_VERSION}"
  PACKAGES=$(list_packages)

  #Add additional files
  cp -a ./${BUILD_OS_NAME}/*.sh ${SRC_DIR}/

  pushd ${SRC_DIR} >/dev/null
  cat <<EOF >Containerfile
FROM ${BASE_IMAGE_NAME} AS base

# Zypper Install
RUN zypper --non-interactive refresh && \
  zypper --non-interactive update -y && \
  zypper --non-interactive install -y ${PACKAGES} 

EOF

  # Containerfile excerpts
  add_containerfile_excerpts >>Containerfile

  cat <<EOF >>Containerfile

# Sign it
RUN date +%Y-%m-%dT%H:%M:%S >/etc/skybox-release.build
CMD ["bash"]
EOF

  # Update Slurm version
  sed -i "s,@SLURM_VERSION@,${SLURM_VERSION},g" Containerfile
  cat Containerfile

  # BUILD
  podman build --file Containerfile --tag ${BUILD_IMAGE_NAME} .

  popd >/dev/null
}

function build_container_image() {

  echo "Building Container Image ${BUILD_IMAGE_NAME}"

  case ${BUILD_OS_NAME} in
    opensuse)
      build_container_image_opensuse
      ;;
    *)	    
      echo "ERROR (build_container_image): unsupported OS ${BUILD_OS_NAME} " >&2  
      return 1
      ;;
  esac	  

  # check image existence
  if podman image exists ${BUILD_IMAGE_NAME}
  then
    return 0
  else
    echo "ERROR (build_container_image): cannot build ${BUILD_IMAGE_NAME}" >&2  
    return 1
  fi
}

function check_build_container_image() {
  local IMAGE_NAME_PREFIX="skybox-release-build"

  if [ -z "${SLURM_VERSION}" ]
  then
    echo "ERROR (check_build_container): \$SLURM_VERSION must be set at this stage." >&2  
    return 1
  fi

  if [ -z "${BUILD_OS}" ]
  then
    echo "ERROR (check_build_container): \$BUILD_OS must be set at this stage."	>&2  
    return 1
  fi

  if [ -z "${ARCH}" ]
  then
    echo "ERROR (check_build_container): \$ARCH must be set at this stage." >&2
    return 1
  fi

  # check image existence
  export BUILD_IMAGE_NAME="${IMAGE_NAME_PREFIX}-slurm-${SLURM_VERSION}-${BUILD_OS}-${ARCH}"
  if podman image exists ${BUILD_IMAGE_NAME}
  then
    return 0	  
  else
    build_container_image
  fi
}
