#!/bin/bash

# The generator versions the committed bindings are made with. Others
# change the generated files, so the script refuses them.
# protoc: the version pkg/apis/*.pb.go's headers name.
PROTOC_VERSION="3.21.12"
# protoc-gen-go: go.mod's google.golang.org/protobuf.
PROTOC_GEN_GO_VERSION="v1.28.1"
# MemeLabs/protobuf's ts, tsrpc and gorpc plugins (go.mod's and
# package.json's v0.3.5) report no version, so they go unchecked.

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)"

# require <command> <expected version> <where the pin comes from>
require() {
  local found
  found="$("$1" --version 2> /dev/null | awk '{print $2}')"
  if [ "$found" != "$2" ]; then
    echo "protogen.sh: $1 $2 expected ($3), found ${found:-none}" >&2
    exit 1
  fi
}

# The checks look the generators up on the PATH protoc runs with.
PATH="${pwd}:$PATH" require protoc "$PROTOC_VERSION" \
  "PROTOC_VERSION, the version pkg/apis/*.pb.go's headers name"
PATH="${pwd}:$PATH" require protoc-gen-go "$PROTOC_GEN_GO_VERSION" \
  "PROTOC_GEN_GO_VERSION, go.mod's google.golang.org/protobuf"

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
