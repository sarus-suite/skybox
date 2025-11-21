#!/usr/bin/bash

SUPPORTED_SLURM_VERSION="24.05.3-1,25.05.4-1"
DEFAULT_SLURM_VERSION="25.05.4-1"

function check_slurm_version() {

  # Set defaults
  if [ -z "${SLURM_VERSION}" ]
  then
    export SLURM_VERSION=${DEFAULT_SLURM_VERSION}
  fi

  if ! (echo ",${SUPPORTED_SLURM_VERSION}," | grep -q ",${SLURM_VERSION},")
  then
    echo "ERROR: Unsupported slurm-version, please choose one from: ${SUPPORTED_SLURM_VERSION}"
    return 1
  fi
}
