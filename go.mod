module github.com/MemeLabs/go-ppspp

go 1.13

require (
	github.com/aead/ecdh v0.2.0
	github.com/bwesterb/go-ristretto v1.1.1
	github.com/davecgh/go-spew v1.1.1
	github.com/emirpasic/gods v1.12.0
	github.com/golang/protobuf v1.4.2
	github.com/gorilla/mux v1.7.4
	github.com/gorilla/websocket v1.4.2
	github.com/hashicorp/golang-lru v0.5.4
	github.com/kr/pretty v0.2.0 // indirect
	github.com/lucas-clemente/quic-go v0.15.7 // indirect
	github.com/minio/blake2b-simd v0.0.0-20160723061019-3f5f724cb5b1
	github.com/nareix/joy4 v0.0.0-20181022032202-3ddbc8f9d431
	github.com/onsi/gomega v1.10.1 // indirect
	github.com/petar/GoLLRB v0.0.0-20190514000832-33fb24c13b99
	github.com/pion/dtls v1.5.4
	github.com/pion/webrtc/v2 v2.2.14
	github.com/prometheus/client_golang v1.6.0
	github.com/prometheus/common v0.10.0
	go.etcd.io/bbolt v1.3.4
	go.uber.org/zap v1.15.0
	golang.org/x/crypto v0.0.0-20200510223506-06a226fb4e37
	golang.org/x/sync v0.0.0-20200317015054-43a5402ce75a
	golang.org/x/tools v0.0.0-20200522201501-cb1345f3a375
	google.golang.org/protobuf v1.23.0
	gopkg.in/yaml.v2 v2.3.0
	lukechampine.com/uint128 v1.0.0
)

replace github.com/nareix/joy4 => github.com/Seize/joy4 v0.0.8

replace github.com/prometheus/client_golang => ./vendor/client_golang
