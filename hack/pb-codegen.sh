#!/bin/bash

set -e
pushd $(/bin/pwd) >/dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)/.."

SCHEMA_DIR="schema"
JS_DIR="src/lib/pb"
GO_DIR="pkg/pb"
SWIFT_DIR="ios/App/App/ProtoBuf"
JAVA_DIR="android/app/src/main/java/"

INPUT_DIR="$SCHEMA_DIR/sanitized"
rm -rf $INPUT_DIR
mkdir -p $INPUT_DIR
hack/remove-services.sh $INPUT_DIR "$(find $SCHEMA_DIR -type f)"

SOURCES="$(ls $INPUT_DIR | sort)"
REL_SOURCES="$(find $INPUT_DIR -type f | sort)"

hack/ts-preprocess.sh /tmp/pbjstemp.proto $REL_SOURCES
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
    -I $INPUT_DIR \
    $SOURCES

if command -v protoc-gen-swift &> /dev/null
then
    protoc \
        --swift_out $SWIFT_DIR \
        --swift_opt Visibility=Public \
        -I $INPUT_DIR \
        $SOURCES

    bash ./hack/swift-codegen.sh $REL_SOURCES
fi

popd > /dev/null
