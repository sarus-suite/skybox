#!/usr/bin/bash

cd $(dirname $0)


ARCH="$(uname -m)"
test -e $ARCH || ln -s . $ARCH
mkdir -p ${PWD}/rpm
rpmbuild --target=$ARCH --clean -ba -D"_topdir ${PWD}/rpm"  ./${PRODUCT}.spec
