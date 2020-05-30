#! /bin/bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)"

FILE="../src/lib/pb/register_types.ts"

echo "// GENERATED CODE -- DO NOT EDIT!" >${FILE}
echo "import * as pb from \"./pb\";" >>${FILE}
echo "import { registerType } from \"./registry\";" >>${FILE}
echo "" >>${FILE}

awk '/^message/ { { printf "registerType(\"%s\", pb.%s);\n", $2, $2}; }' \
	../schema/api.proto >>${FILE}

popd > /dev/null
