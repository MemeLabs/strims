go build -o protoc-gen-ts ./cmd/ts/
go build -o protoc-gen-gorpc ./cmd/gorpc/
go build -o protoc-gen-tsrpc ./cmd/tsrpc/

PATH="$PATH:${pwd}" protoc \
  --proto_path=../../schema \
  --go_out=../../pkg/apis \
  --go_opt=module=github.com/MemeLabs/go-ppspp/pkg/apis \
  --gorpc_out=../../pkg/apis \
  --gorpc_opt=module=github.com/MemeLabs/go-ppspp/pkg/apis \
  --ts_out=../../src/lib/test/test \
  --tsrpc_out=../../src/lib/test/test \
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
