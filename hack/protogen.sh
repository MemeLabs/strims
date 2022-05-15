#!/bin/bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)"

SOURCES="$(find ../schema -iname *.proto)"
PATH="${pwd}:$PATH" protoc \
  --proto_path=../schema \
  --go_out=../pkg/apis \
  --go_opt=module=github.com/MemeLabs/strims/pkg/apis \
  --gorpc_out=../pkg/apis \
  --gorpc_opt=module=github.com/MemeLabs/strims/pkg/apis \
  --ts_out=../src/apis \
  --tsrpc_out=../src/apis \
  $SOURCES

popd > /dev/null
