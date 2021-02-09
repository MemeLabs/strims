#!/bin/bash

SOURCES="$(find ../schema -iname *.proto)"
PATH="${pwd}:$PATH" protoc \
  --proto_path=../schema \
  --go_out=../pkg/apis \
  --go_opt=module=github.com/MemeLabs/go-ppspp/pkg/apis \
  --gorpc_out=../pkg/apis \
  --gorpc_opt=module=github.com/MemeLabs/go-ppspp/pkg/apis \
  --ts_out=../src/apis \
  --tsrpc_out=../src/apis \
  $SOURCES
