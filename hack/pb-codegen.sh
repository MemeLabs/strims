#! /bin/bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)"

PROTOC_GEN_TS_PATH="../node_modules/.bin/protoc-gen-ts"
SCHEMA_DIR="../schema"
JS_DIR="../src/service"
GO_DIR="../pkg/service"

protoc \
    --plugin "protoc-gen-ts=${PROTOC_GEN_TS_PATH}" \
    --js_out "import_style=commonjs,binary:${JS_DIR}" \
    --ts_out $JS_DIR \
    -I $SCHEMA_DIR \
    rpc.proto api.proto

protoc \
    --go_out $GO_DIR \
    -I $SCHEMA_DIR \
    rpc.proto api.proto

popd > /dev/null
