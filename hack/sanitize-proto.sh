#!/bin/bash

SCHEMA_DIR="schema"
INPUT_DIR="$SCHEMA_DIR/sanitized"
rm -rf $INPUT_DIR
mkdir -p $INPUT_DIR
hack/remove-services.sh $INPUT_DIR "$(find $SCHEMA_DIR -type f)"
