// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package peer

import (
	"context"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/pkg/rpcutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

const (
	RPCClientRetries = 3
	RPCClientBackoff = 2
	RPCClientDelay   = 100 * time.Millisecond
	RPCClientTimeout = time.Second
)

type PeerHandler interface {
	AddPeer(id uint64, p *vnic.Peer, s *rpc.Server, c rpc.Caller)
	RemovePeer(id uint64)
}

type Control interface {
	HandlePeer(peer *vnic.Peer)
	AddPeerHandler(h PeerHandler)
}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	observers *event.Observers,
	vpn *vpn.Host,
	handlers ...PeerHandler,
) Control {
	return &control{
		logger:    logger,
		observers: observers,
		qosc:      vpn.VNIC().QOS().AddClass(1),
		handlers:  handlers,
	}
}

// PeerControl ...
type control struct {
	logger    *zap.Logger
	observers *event.Observers
	qosc      *qos.Class

	lock     sync.Mutex
	nextID   uint64
	handlers []PeerHandler
}

func (t *control) AddPeerHandler(h PeerHandler) {
	t.lock.Lock()
	defer t.lock.Unlock()
	t.handlers = append(t.handlers, h)
}

func (t *control) HandlePeer(peer *vnic.Peer) {
	id := atomic.AddUint64(&t.nextID, 1)

	logger := t.logger.With(zap.Stringer("host", peer.HostID()))
	rw0, rw1 := peer.ChannelPair(vnic.PeerRPCClientPort, vnic.PeerRPCServerPort, t.qosc)

	c, err := rpc.NewClient(logger, &rpc.RWFDialer{
		Logger:           logger,
		ReadWriteFlusher: rw0,
	})
	if err != nil {
		logger.Info("creating peer rpc client failed", zap.Error(err))
		return
	}

	rc := rpc.Caller(rpcutil.NewClientRetrier(c, RPCClientRetries, RPCClientBackoff, RPCClientDelay, RPCClientTimeout))
	if logger.Core().Enabled(zap.DebugLevel) {
		rc = rpcutil.NewClientLogger(rc, logger)
	}

	s := rpc.NewServer(logger, &rpc.RWFDialer{
		Logger:           logger,
		ReadWriteFlusher: rw1,
	})

	t.lock.Lock()
	for _, h := range t.handlers {
		h.AddPeer(id, peer, s, rc)
	}
	t.lock.Unlock()

	t.observers.EmitLocal(event.PeerAdd{
		ID:     id,
		HostID: peer.HostID(),
	})

	go func() {
		logger.Debug("peer rpc server listening")
		err := s.Listen(context.Background())
		if err != nil {
			logger.Debug("peer rpc server closed with error", zap.Error(err))
		}

		t.observers.EmitLocal(event.PeerRemove{
			ID:     id,
			HostID: peer.HostID(),
		})

		t.lock.Lock()
		for _, h := range t.handlers {
			h.RemovePeer(id)
		}
		t.lock.Unlock()

		c.Close()
	}()
}
