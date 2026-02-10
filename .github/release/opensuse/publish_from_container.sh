#!/usr/bin/bash

cd "$(dirname $0)/../../../"
DIST_DIR="/mnt/.github/release/dist/"

cd /tmp/
#curl -sOL https://gitlab.com/gitlab-org/cli/-/releases/v1.78.2/downloads/glab_1.78.2_linux_amd64.rpm
#zypper install --allow-unsigned-rpm -y bind-utils openssh ./glab_1.78.2_linux_amd64.rpm

echo "ENVIRONMENT:"
env
echo "OS:"
cat /etc/os-release | grep PRETTY

cd /mnt
VERSION_TAG=$(git describe --tags)
GIT_BRANCH=$(git branch --show-current)
echo "TAG:"
echo $VERSION_TAG
if [ -z "${VERSION_TAG}" ]
then
    echo "ERROR: cannot gather RELEASE TAG"
    echo "VERSION_TAG=${VERSION_TAG}"	
    echo "BRANCH=${GIT_BRANCH}"
    exit 1
fi
cd /tmp

echo "GATHER SSH KNOWN HOSTS for git.cscs.ch"
mkdir -p ~/.ssh
ssh-keyscan git.cscs.ch >> ~/.ssh/known_hosts
ssh-keyscan $(host git.cscs.ch | tail -n 1 | awk '{print $NF}') >> ~/.ssh/known_hosts
echo "GIT CLONE"
git clone git@git.cscs.ch:chesim/internal-skybox.git
cd internal-skybox
echo "TRIGGER GITLAB PIPELINE"
glab ci run -b main --variables-env CI_DELIVER_JFROG:true --variables-env CI_RELEASE_TO_PUBLISH:${VERSION_TAG}
