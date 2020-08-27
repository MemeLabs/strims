#! /bin/bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)/.."

SCHEMA_DIR="schema"
JS_DIR="src/lib/pb"
GO_DIR="pkg/pb"
SWIFT_DIR="ios/App/App/ProtoBuf"

npx pbjs \
    -t static-module \
    -w es6 \
    --force-number \
    -o "${JS_DIR}/pb.js" \
    -p $SCHEMA_DIR \
    rpc.proto api.proto
npx pbts \
    -o "${JS_DIR}/pb.d.ts" \
    "${JS_DIR}/pb.js"

bash ./hack/ts-codegen.sh

protoc \
    --go_out $GO_DIR \
    --go_opt=paths=source_relative \
    -I $SCHEMA_DIR \
    rpc.proto api.proto

protoc \
    --swift_out $SWIFT_DIR \
    --swift_opt Visibility=Public \
    -I $SCHEMA_DIR \
    rpc.proto api.proto

bash ./hack/swift-codegen.sh

popd > /dev/null
