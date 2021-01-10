go build -o protoc-gen-ts .

PATH="$PATH:${pwd}" protoc \
  -I ../../schema \
  --ts_out="../../src/lib/test/test" \
  ../../schema/type/*.proto \
  ../../schema/vpn/v1/*.proto \
  ../../schema/video/v1/*.proto \
  ../../schema/transfer/v1/*.proto \
  ../../schema/rpc/v1/*.proto \
  ../../schema/profile/*.proto \
  ../../schema/network/v1/*.proto \
  ../../schema/network/v1/ca/*.proto \
  ../../schema/network/v1/bootstrap/*.proto \
  ../../schema/funding/v1/*.proto \
  ../../schema/debug/v1/*.proto \
  ../../schema/dao/*.proto \
  ../../schema/chat/v1/*.proto

# npx prettier --write "../../src/lib/test/test/**/*.ts"
