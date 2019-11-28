#! /bin/bash

FILE="../src/service/types.ts"

echo "// GENERATED CODE -- DO NOT EDIT!" >${FILE}
echo "import * as api_pb from \"./api_pb\";" >>${FILE}
echo "import { registerType } from \"./rpc_host\";" >>${FILE}
echo "" >>${FILE}

awk '/message/ { { printf "registerType(\"%s\", api_pb.%s)\n", $2, $2}; }' \
	../schema/api.proto >>"src/service/types.ts"
