#!/bin/bash

SCHEMA_DIR="schema"
INPUT_DIR="$SCHEMA_DIR/sanitized"
rm -rf $INPUT_DIR
mkdir -p $INPUT_DIR
rm $INPUT_DIR/*.proto || echo "nothing to clean"
hack/remove-services.sh $INPUT_DIR "$(find $SCHEMA_DIR -type f)"
