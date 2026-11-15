#!/bin/bash

# The generator versions the committed bindings are made with. Others
# change the generated files, so the script refuses them.
# protoc: the version pkg/apis/*.pb.go's headers name.
PROTOC_VERSION="3.21.12"
# protoc-gen-go: go.mod's google.golang.org/protobuf, read from go.mod so
# the generator moves with the runtime.
PROTOC_GEN_GO_VERSION="$(awk '$1 == "google.golang.org/protobuf" {print $2}' \
  "$(dirname "$(realpath "$0")")/../go.mod")"
# MemeLabs/protobuf's ts, tsrpc and gorpc plugins (go.mod's
# github.com/MemeLabs/protobuf, package.json's @memelabs/protobuf) report
# no version, so they go unchecked.

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

# Every generated file ends .pb.ts or .pb.go and no hand-written one does
# (src/apis/client.ts, pkg/apis/client.go), so clearing them first leaves no
# bindings behind for a removed or renamed proto.
find ../src/apis ../pkg/apis \( -name '*.pb.ts' -o -name '*.pb.go' \) -delete

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
