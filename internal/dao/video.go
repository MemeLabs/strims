package dao

import videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"

const (
	_ = iota + videoNS
	videoChannelNS
	videoChannelKeyNS
	videoIngressConfigNS
)

var VideoIngressConfig = NewSingleton(
	videoIngressConfigNS,
	&SingletonOptions[videov1.VideoIngressConfig]{
		DefaultValue: &videov1.VideoIngressConfig{
			ServerAddr: "127.0.0.1:1935",
		},
	},
)
