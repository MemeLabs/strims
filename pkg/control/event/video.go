package event

import "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"

// VideoIngressConfigUpdate ...
type VideoIngressConfigUpdate struct {
	Config *video.VideoIngressConfig
}

// VideoChannelUpdate ...
type VideoChannelUpdate struct {
	Channel *video.VideoChannel
}

// VideoChannelRemove ...
type VideoChannelRemove struct {
	ID uint64
}
