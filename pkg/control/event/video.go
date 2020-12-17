package event

import "github.com/MemeLabs/go-ppspp/pkg/pb"

// VideoIngressChannelUpdate ...
type VideoIngressChannelUpdate struct {
	Channel *pb.VideoIngressChannel
}

// VideoIngressChannelRemove ...
type VideoIngressChannelRemove struct {
	ID uint64
}
