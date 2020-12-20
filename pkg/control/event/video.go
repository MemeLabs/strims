package event

import "github.com/MemeLabs/go-ppspp/pkg/pb"

// VideoChannelUpdate ...
type VideoChannelUpdate struct {
	Channel *pb.VideoChannel
}

// VideoChannelRemove ...
type VideoChannelRemove struct {
	ID uint64
}
