#!/bin/bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)"

go build -o protoc-gen-gorpc ./protogen/cmd/gorpc
go build -o protoc-gen-tsrpc ./protogen/cmd/tsrpc
go build -o protoc-gen-ts ./protogen/cmd/ts
sudo mv protoc-gen-* /usr/local/bin/

popd > /dev/null
