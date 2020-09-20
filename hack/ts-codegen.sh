#!/usr/bin/env bash

set -e
pushd $(/bin/pwd) > /dev/null

BASE="$(realpath $0)"
cd "$(dirname $BASE)/.."

FILE="src/lib/pb/register_types.ts"

echo "// GENERATED CODE -- DO NOT EDIT!" > ${FILE}
echo "import * as pb from \"./pb\";" >> ${FILE}
echo "import { registerType } from \"./registry\";" >> ${FILE}
echo "" >> ${FILE}

cat $* | awk '/^message/ { { printf "registerType(\"%s\", pb.%s);\n", $2, $2}; }' >> ${FILE}

popd > /dev/null
