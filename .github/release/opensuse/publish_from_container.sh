#!/usr/bin/bash

VERSION_TAG="$1"
cd "$(dirname $0)/../../../"
DIST_DIR=".github/release/dist/"

cd /tmp/
curl -sOL https://gitlab.com/gitlab-org/cli/-/releases/v1.78.2/downloads/glab_1.78.2_linux_amd64.rpm
zypper install --allow-unsigned-rpm -y openssh ./glab_1.78.2_linux_amd64.rpm

echo "ENVIRONMENT:"
env
echo "OS"
cat /etc/os-release | grep PRETTY

echo "GATHER SSH KNOWN HOSTS"
mkdir -p ~/.ssh
ssh-keyscan git.cscs.ch >> ~/.ssh/known_hosts
echo "GIT CLONE"
git clone git@git.cscs.ch:chesim/internal-skybox.git
cd internal-skybox
echo "TRIGGER GITLAB PIPELINE"
glab ci run -b main --variables-env CI_DELIVER_JFROG:true --variables-env CI_RELEASE_TO_PUBLISH:${VERSION_TAG}
