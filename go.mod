module github.com/MemeLabs/go-ppspp

go 1.13

require (
	github.com/MemeLabs/chat-parser v1.0.1
	github.com/Microsoft/go-winio v0.4.14 // indirect
	github.com/aead/ecdh v0.2.0
	github.com/avast/retry-go v2.6.0+incompatible
	github.com/bwesterb/go-ristretto v1.1.1
	github.com/chromedp/cdproto v0.0.0-20200706151146-b7f349b11751
	github.com/chromedp/chromedp v0.5.3
	github.com/davecgh/go-spew v1.1.1
	github.com/docker/distribution v2.7.1+incompatible // indirect
	github.com/docker/docker v1.13.1
	github.com/docker/go-connections v0.4.0
	github.com/docker/go-units v0.4.0 // indirect
	github.com/emirpasic/gods v1.12.0
	github.com/gobwas/pool v0.2.1 // indirect
	github.com/gobwas/ws v1.0.3 // indirect
	github.com/golang/geo v0.0.0-20200319012246-673a6f80352d
	github.com/golang/protobuf v1.4.2
	github.com/gorilla/mux v1.7.4
	github.com/gorilla/websocket v1.4.2
	github.com/hashicorp/golang-lru v0.5.4
	github.com/kr/pretty v0.2.0 // indirect
	github.com/lucas-clemente/quic-go v0.17.2 // indirect
	github.com/marten-seemann/qtls v0.10.0 // indirect
	github.com/minio/blake2b-simd v0.0.0-20160723061019-3f5f724cb5b1
	github.com/nareix/joy4 v0.0.0-20200507095837-05a4ffbb5369
	github.com/nareix/joy5 v0.0.0-20200409150540-6c2a804a2816
	github.com/onsi/gomega v1.10.1 // indirect
	github.com/opencontainers/go-digest v1.0.0 // indirect
	github.com/petar/GoLLRB v0.0.0-20190514000832-33fb24c13b99
	github.com/pion/dtls v1.5.4 // indirect
	github.com/pion/transport v0.10.1 // indirect
	github.com/pion/turn/v2 v2.0.4 // indirect
	github.com/pion/webrtc/v2 v2.2.18
	github.com/prometheus/client_golang v1.7.1
	github.com/prometheus/common v0.10.0
	github.com/prometheus/procfs v0.1.3 // indirect
	github.com/stretchr/testify v1.6.1
	github.com/tj/assert v0.0.3
	go.etcd.io/bbolt v1.3.5
	go.uber.org/zap v1.15.0
	golang.org/dl v0.0.0-20200611200201-72429b14455f // indirect
	golang.org/x/crypto v0.0.0-20200707235045-ab33eee955e0
	golang.org/x/net v0.0.0-20200707034311-ab3426394381 // indirect
	golang.org/x/sync v0.0.0-20200625203802-6e8e738ad208
	golang.org/x/sys v0.0.0-20200625212154-ddb9806d33ae // indirect
	golang.org/x/text v0.3.3 // indirect
	golang.org/x/time v0.0.0-20200630173020-3af7569d3a1e
	golang.org/x/tools v0.0.0-20200708183856-df98bc6d456c
	google.golang.org/protobuf v1.25.0
	gopkg.in/yaml.v2 v2.3.0
	lukechampine.com/uint128 v1.0.0
	mvdan.cc/xurls/v2 v2.2.0
)

replace github.com/prometheus/client_golang => ./vendor/client_golang
