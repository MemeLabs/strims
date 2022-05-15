// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"time"

	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
)

type swarmScheduler interface {
	store.Subscriber
	ChannelScheduler(p peerTaskQueue, cw codecMessageWriter) channelScheduler
	CloseChannel(p peerTaskQueue)
	Run(t timeutil.Time)
}

type peerTaskRunner interface {
	PunchTicket(v uint64) bool
	Write() (int, error)
	WriteData(b binmap.Bin, t timeutil.Time, pri peerPriority) (int, error)
}

type channelScheduler interface {
	peerTaskRunner
	HandleHandshake(liveWindow uint32) error
	HandleAck(b binmap.Bin, delaySample time.Duration) error
	HandleData(b binmap.Bin, t timeutil.Time, valid bool) error
	HandleHave(b binmap.Bin) error
	HandleRequest(b binmap.Bin, t timeutil.Time) error
	HandleCancel(b binmap.Bin) error
	HandlePing(nonce uint64) error
	HandlePong(nonce uint64) error
	HandleChoke() error
	HandleUnchoke() error
	HandleStreamRequest(s codec.Stream, b binmap.Bin) error
	HandleStreamCancel(s codec.Stream) error
	HandleStreamOpen(s codec.Stream, b binmap.Bin) error
	HandleStreamClose(s codec.Stream) error
	HandleMessageEnd() error
}

type peerTaskQueue interface {
	ID() []byte
	Enqueue(w peerTaskRunner)
	EnqueueNow(w peerTaskRunner)
	PushData(w peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority)
	PushFrontData(w peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority)
	RemoveData(w peerTaskRunner, b binmap.Bin, pri peerPriority)
	RemoveRunner(w peerTaskRunner)
}

type codecMessageWriter interface {
	Available() int
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
	WriteChoke(m codec.Choke) (int, error)
	WriteUnchoke(m codec.Unchoke) (int, error)
	WriteStreamRequest(m codec.StreamRequest) (int, error)
	WriteStreamCancel(m codec.StreamCancel) (int, error)
	WriteStreamOpen(m codec.StreamOpen) (int, error)
	WriteStreamClose(m codec.StreamClose) (int, error)
}

type SchedulingMethod int

func (m SchedulingMethod) swarmScheduler(logger *zap.Logger, s *Swarm) swarmScheduler {
	return newPeerSwarmScheduler(logger, s)

	// TODO: do we need this?
	// switch m {
	// case SeedSchedulingMethod:
	// 	return newSeedSwarmScheduler(logger, s)
	// case PeerSchedulingMethod:
	// 	return newPeerSwarmScheduler(logger, s)
	// default:
	// 	panic("invalid sheduling method")
	// }
}

const (
	SeedSchedulingMethod SchedulingMethod = iota + 1
	PeerSchedulingMethod
)

type DeliveryMode int

const (
	_ DeliveryMode = iota
	LowLatencyDeliveryMode
	BestEffortDeliveryMode
	MandatoryDeliveryMode
)

func streamBinOffset(s codec.Stream) binmap.Bin {
	return binmap.Bin(s * 2)
}
