go build -o protoc-gen-ts .

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/type/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/vpn/v1/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/video/v1/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/transfer/v1/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/rpc/v1/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/profile/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/network/v1/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/network/v1/ca/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/network/v1/bootstrap/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/funding/v1/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/debug/v1/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/dao/*.proto

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/chat/v1/*.proto

# npx prettier --write "../../src/lib/test/test/**/*.ts"
