package dao

import (
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + videoNS
	videoChannelNS
	videoChannelKeyNS
	videoIngressConfigNS
	videoHLSEgressConfigNS
)

var VideoIngressConfig = NewSingleton(
	videoIngressConfigNS,
	&SingletonOptions[videov1.VideoIngressConfig, *videov1.VideoIngressConfig]{
		DefaultValue: &videov1.VideoIngressConfig{
			ServerAddr: "127.0.0.1:1935",
		},
		ObserveChange: func(m, p *videov1.VideoIngressConfig) proto.Message {
			return &videov1.VideoIngressConfigChangeEvent{IngressConfig: m}
		},
	},
)

var HLSEgressConfig = NewSingleton(
	videoHLSEgressConfigNS,
	&SingletonOptions[videov1.HLSEgressConfig, *videov1.HLSEgressConfig]{
		DefaultValue: &videov1.HLSEgressConfig{},
		ObserveChange: func(m, p *videov1.HLSEgressConfig) proto.Message {
			return &videov1.HLSEgressConfigChangeEvent{EgressConfig: m}
		},
	},
)
