#!/bin/bash -x

# Canonicalize SLURM_VERSION to a distribution version.
echo "SLURM_VERSION=${SLURM_VERSION}"
SLURM_GIT_TAG=${SLURM_VERSION%-*}

slurm_tar_file=slurm-${SLURM_GIT_TAG}.tar.bz2
slurm_url=https://download.schedmd.com/slurm/${slurm_tar_file}

#
# Download slurm tarball and unpack it
#
if true; then

    mkdir -p /opt/src || exit 1
    (
        cd /opt/src

        if ! stat $slurm_tar_file; then
            echo "=== downloading slurm ${SLURM_GIT_TAG} from ${slurm_url}"
            curl --fail --output ${slurm_tar_file} ${slurm_url} || exit 1
        fi

        echo "=== unpacking $slurm_tar_file"
        tar -xjf ${slurm_tar_file} || exit 1
    )

fi

#
# Remove any old build directory.
# Run configure, make, make install
#

stat /opt/build/slurm-${SLURM_GIT_TAG} && rm -rf /opt/build/slurm-${SLURM_GIT_TAG}
mkdir -p /opt/build/slurm-${SLURM_GIT_TAG} || exit 1
(
    cd /opt/build/slurm-${SLURM_GIT_TAG}
    /opt/src/slurm-${SLURM_GIT_TAG}/configure --help
    /opt/src/slurm-${SLURM_GIT_TAG}/configure \
        --disable-dependency-tracking

    make && make install
)
