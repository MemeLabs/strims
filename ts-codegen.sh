#!/bin/env bash
echo "import { registerType } from \"./rpc_host\";" >>"src/service/types.ts"
sed 's/^message //; s/{}//; /[};]$/d; s/{//' schema/api.proto |
	awk '/./{ printf "registerType(%s, api_pb.%s)\n", $1, $1}' >>"src/service/types.ts"
