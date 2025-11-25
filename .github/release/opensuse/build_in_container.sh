#!/usr/bin/bash

cd "$(dirname $0)/../../../"
DIST_DIR=".github/release/dist/"

rm -rf ${DIST_DIR}

if [ -z "${VERSION}" ]
then
  VERSION=$(git describe --always --tags | tr '-' '_' | sed 's/_.*_/-/')
  if ! ( echo $VERSION | grep -Eq "^v[0-9]+\.[0-9]+\.[0-9]+" )
  then	 
    RELEASE_VERSION="0.0.0-${VERSION}"
  else
    RELEASE_VERSION=${VERSION#v}
  fi
else
  RELEASE_VERSION=${VERSION#v}
fi

PRODUCT="skybox"
TARGET_NAME="release"

sed -i -E -e "/^name = \"${PRODUCT}\"/,/^version \=/ s/^version =.*$/version = \"${RELEASE_VERSION#v}\"/1" Cargo.toml

cargo build --release

mkdir ${DIST_DIR}
cp target/${TARGET_NAME}/lib${PRODUCT}.so ${DIST_DIR}/
