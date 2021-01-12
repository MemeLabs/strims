#!/usr/bin/env bash

set -e
pushd $(/bin/pwd) >/dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)/.."

SCHEMA_DIR="schema"
SWIFT_DIR="ios/App/App/ProtoBuf"

INPUT_DIR="$SCHEMA_DIR/sanitized"
rm -rf $INPUT_DIR
mkdir -p $INPUT_DIR
hack/remove-services.sh $INPUT_DIR "$(find $SCHEMA_DIR -type f)"

SOURCES="$(ls $INPUT_DIR | sort)"
REL_SOURCES="$(find $INPUT_DIR -type f | sort)"

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
