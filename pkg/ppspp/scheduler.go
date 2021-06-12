package ppspp

import (
	"math"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"go.uber.org/zap"
)

type SwarmScheduler interface {
	store.Subscriber
	ChannelScheduler(p peerThing, cw channelWriterThing) ChannelScheduler
	CloseChannel(p peerThing)
	Run(t time.Time)
}

type PeerWriter interface {
	Write(maxBytes int) (int, error)
	WriteData(maxBytes int, b binmap.Bin, t time.Time, pri peerPriority) (int, error)
}

type ChannelScheduler interface {
	PeerWriter
	HandleHandshake(liveWindow uint32) error
	HandleAck(b binmap.Bin, delaySample time.Duration) error
	HandleData(b binmap.Bin, t time.Time, valid bool) error
	HandleHave(b binmap.Bin) error
	HandleRequest(b binmap.Bin, t time.Time) error
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

type peerThing interface {
	ID() []byte
	addReceivedBytes(uint64, time.Time)
	enqueue(t *PeerWriterQueueTicket, w PeerWriter)
	enqueueNow(t *PeerWriterQueueTicket, w PeerWriter)
	pushData(w PeerWriter, b binmap.Bin, t time.Time, pri peerPriority)
	pushFrontData(w PeerWriter, b binmap.Bin, t time.Time, pri peerPriority)
	removeData(w PeerWriter, b binmap.Bin, pri peerPriority)
	closeChannel(w PeerWriter)
}

type channelWriterThing interface {
	Resize(int) error
	Len() int
	Flush() error
	Reset()
	WriteHandshake(m codec.Handshake) (int, error)
	WriteAck(m codec.Ack) (int, error)
	WriteHave(m codec.Have) (int, error)
	WriteData(m codec.Data) (int, error)
	WriteIntegrity(m codec.Integrity) (int, error)
	WriteSignedIntegrity(m codec.SignedIntegrity) (int, error)
	WriteRequest(m codec.Request) (int, error)
	WritePing(m codec.Ping) (int, error)
	WritePong(m codec.Pong) (int, error)
	WriteCancel(m codec.Cancel) (int, error)
	WriteStreamRequest(m codec.StreamRequest) (int, error)
	WriteStreamCancel(m codec.StreamCancel) (int, error)
	WriteStreamOpen(m codec.StreamOpen) (int, error)
	WriteStreamClose(m codec.StreamClose) (int, error)
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

func streamBinOffset(s codec.Stream) binmap.Bin {
	return binmap.Bin(s * 2)
}

var epochTime = time.Unix(0, 0)
var maxTime = time.Unix(0, math.MaxInt64)
