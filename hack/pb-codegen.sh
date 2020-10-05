#! /bin/bash

set -e
pushd $(/bin/pwd) >/dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)/.."

SCHEMA_DIR="schema"
JS_DIR="src/lib/pb"
GO_DIR="pkg/pb"
SWIFT_DIR="ios/App/App/ProtoBuf"
JAVA_DIR="android/app/src/main/java/"

SOURCES="$(ls $SCHEMA_DIR)"
REL_SOURCES="$(find $SCHEMA_DIR -type f)"

hack/ts-preprocess.sh $REL_SOURCES /tmp/pbjstemp.proto
npx pbjs \
    -t static-module \
    -w es6 \
    --force-number \
    -o "${JS_DIR}/pb.js" \
    /tmp/pbjstemp.proto
npx pbts \
    -o "${JS_DIR}/pb.d.ts" \
    "${JS_DIR}/pb.js"
bash ./hack/ts-codegen.sh $REL_SOURCES

protoc \
    --go_out $GO_DIR \
    --go_opt=paths=source_relative \
    -I $SCHEMA_DIR \
    $SOURCES

if command -v protoc-gen-swift &> /dev/null
then
    protoc \
        --swift_out $SWIFT_DIR \
        --swift_opt Visibility=Public \
        -I $SCHEMA_DIR \
        $SOURCES

    bash ./hack/swift-codegen.sh $REL_SOURCES
fi

popd > /dev/null
