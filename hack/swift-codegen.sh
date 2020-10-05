#!/bin/bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)/.."

FILE="ios/App/App/ProtoBuf/types.swift"

echo "// GENERATED CODE -- DO NOT EDIT!" > ${FILE}
echo "// swift-format-ignore-file" >> ${FILE}
echo "import SwiftProtobuf" >> ${FILE}
echo "func registerAnyTypes() {" >> ${FILE}

cat $* | awk '/^message/ { { printf "  Google_Protobuf_Any.register(messageType: PB%s.self)\n", $2, $2}; }' >> ${FILE}

echo "}" >> ${FILE}
echo "" >> ${FILE}


popd > /dev/null
