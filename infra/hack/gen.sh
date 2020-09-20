#!/usr/bin/env bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
SCRIPTS_DIR="$(dirname $BASE)"
BASE_DIR="$SCRIPTS_DIR/.."

sqlboiler \
    -c "$SCRIPTS_DIR/sqlboiler.toml" \
    -o "$BASE_DIR/internal/models" \
    --struct-tag-casing "camel" \
    sqlite3

popd > /dev/null
