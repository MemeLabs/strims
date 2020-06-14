module github.com/MemeLabs/go-ppspp

go 1.13

require (
	github.com/Microsoft/go-winio v0.4.14 // indirect
	github.com/aead/ecdh v0.2.0
	github.com/avast/retry-go v2.6.0+incompatible
	github.com/bwesterb/go-ristretto v1.1.1
	github.com/chromedp/cdproto v0.0.0-20200608134039-8a80cdaf865c
	github.com/chromedp/chromedp v0.5.3
	github.com/davecgh/go-spew v1.1.1
	github.com/docker/distribution v2.7.1+incompatible // indirect
	github.com/docker/docker v1.13.1
	github.com/docker/go-connections v0.4.0
	github.com/docker/go-units v0.4.0 // indirect
	github.com/emirpasic/gods v1.12.0
	github.com/gobwas/ws v1.0.3 // indirect
	github.com/golang/protobuf v1.4.2
	github.com/gorilla/mux v1.7.4
	github.com/gorilla/websocket v1.4.2
	github.com/hashicorp/golang-lru v0.5.4
	github.com/kr/pretty v0.2.0 // indirect
	github.com/lucas-clemente/quic-go v0.16.1 // indirect
	github.com/minio/blake2b-simd v0.0.0-20160723061019-3f5f724cb5b1
	github.com/nareix/joy4 v0.0.0-20181022032202-3ddbc8f9d431
	github.com/onsi/gomega v1.10.1 // indirect
	github.com/opencontainers/go-digest v1.0.0 // indirect
	github.com/petar/GoLLRB v0.0.0-20190514000832-33fb24c13b99
	github.com/pion/webrtc/v2 v2.2.14
	github.com/prometheus/client_golang v1.6.0
	github.com/prometheus/common v0.10.0
	github.com/stretchr/testify v1.6.1
	github.com/tj/assert v0.0.3
	go.etcd.io/bbolt v1.3.4
	go.uber.org/zap v1.15.0
	golang.org/x/crypto v0.0.0-20200604202706-70a84ac30bf9
	golang.org/x/net v0.0.0-20200602114024-627f9648deb9 // indirect
	golang.org/x/sync v0.0.0-20200317015054-43a5402ce75a
	golang.org/x/sys v0.0.0-20200602225109-6fdc65e7d980 // indirect
	golang.org/x/tools v0.0.0-20200609164405-eb789aa7ce50
	google.golang.org/protobuf v1.24.0
	gopkg.in/yaml.v2 v2.3.0
	lukechampine.com/uint128 v1.0.0
)

replace github.com/nareix/joy4 => github.com/Seize/joy4 v0.0.8

replace github.com/prometheus/client_golang => ./vendor/client_golang
