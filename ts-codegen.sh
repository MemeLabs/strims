#! /bin/bash
echo "// GENERATED CODE -- DO NOT EDIT!" >"src/service/types.ts"
echo "import * as api_pb from \"./api_pb\";" >>"src/service/types.ts"
echo "import { registerType } from \"./rpc_host\";" >>"src/service/types.ts"
echo "" >>"src/service/types.ts"

awk '/message/ { { printf "registerType(\"%s\", api_pb.%s)\n", $2, $2}; }' \
	schema/api.proto >>"src/service/types.ts"
