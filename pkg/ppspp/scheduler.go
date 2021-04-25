package ppspp

import (
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"go.uber.org/zap"
)

type SwarmScheduler interface {
	store.Subscriber
	ChannelScheduler(p *Peer, cw *channelWriter) ChannelScheduler
	CloseChannel(p *Peer)
	Run(t time.Time)
}

type ChannelScheduler interface {
	PeerWriter
	HandleHandshake(liveWindow uint32) error
	HandleAck(b binmap.Bin, delaySample time.Duration) error
	HandleData(b binmap.Bin, valid bool) error
	HandleHave(b binmap.Bin) error
	HandleRequest(b binmap.Bin) error
	HandleCancel(b binmap.Bin) error
	HandleChoke() error
	HandleUnchoke() error
	HandlePing(nonce uint64) error
	HandlePong(nonce uint64) error
	HandleStreamRequest(s codec.Stream, b binmap.Bin) error
	HandleStreamCancel(s codec.Stream) error
	HandleStreamOpen(s codec.Stream, b binmap.Bin) error
	HandleStreamClose(s codec.Stream) error
	HandleMessageEnd() error
}

type SchedulingMethod int

func (m SchedulingMethod) SwarmScheduler(logger *zap.Logger, s *Swarm) SwarmScheduler {
	switch m {
	case SeedSchedulingMethod:
		return newSeedSwarmScheduler(logger, s)
	case PeerSchedulingMethod:
		return newPeerSwarmScheduler(logger, s)
	default:
		panic("invalid sheduling method")
	}
}

const (
	SeedSchedulingMethod SchedulingMethod = iota + 1
	PeerSchedulingMethod
)

func streamCount(opt SwarmOptions) codec.Stream {
	switch opt.Integrity.ProtectionMethod {
	case integrity.ProtectionMethodMerkleTree:
		return codec.Stream(opt.ChunksPerSignature)
	default:
		return 1
	}
}
