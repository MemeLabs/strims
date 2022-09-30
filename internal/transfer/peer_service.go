// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package transfer

import (
	"context"
	"sync"

	transferv1 "github.com/MemeLabs/strims/pkg/apis/transfer/v1"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"go.uber.org/zap"
)

var _ transferv1.TransferPeerService = (*peerService)(nil)

// Peer ...
type peerService struct {
	logger     *zap.Logger
	ctx        context.Context
	runnerPeer *ppspp.RunnerPeer
	client     *transferv1.TransferPeerClient

	lock        sync.Mutex
	transfers   map[ID]*peerTransfer
	nextChannel uint64
}

func (p *peerService) Announce(ctx context.Context, req *transferv1.TransferPeerAnnounceRequest) (*transferv1.TransferPeerAnnounceResponse, error) {
	id, err := ParseID(req.Id)
	if err != nil {
		return nil, err
	}
	channel, ok := p.AssignPort(id, req.Channel)
	if ok {
		return &transferv1.TransferPeerAnnounceResponse{
			Body: &transferv1.TransferPeerAnnounceResponse_Channel{
				Channel: channel,
			},
		}, nil
	}
	return &transferv1.TransferPeerAnnounceResponse{}, nil
}

func (p *peerService) Close(ctx context.Context, req *transferv1.TransferPeerCloseRequest) (*transferv1.TransferPeerCloseResponse, error) {
	id, err := ParseID(req.Id)
	if err != nil {
		return nil, err
	}
	p.SendClose(id)
	return &transferv1.TransferPeerCloseResponse{}, nil
}

// AssignPort starts a peer transfer when it exists in response to announce from
// peer
func (p *peerService) AssignPort(id ID, peerChannel uint64) (uint64, bool) {
	pt, ok := p.getPeerTransfer(id)
	if !ok {
		return 0, false
	}

	pt.logger.Debug(
		"assigning port",
		zap.Uint64("peerChannel", peerChannel),
	)

	return pt.channel, p.startPeerTransfer(pt, peerChannel)
}

// SendClose stops a peer transfer when it exists in response to close from peer
func (p *peerService) SendClose(id ID) {
	if pt, ok := p.getPeerTransfer(id); ok {
		p.stopPeerTransfer(pt, false)
	}
}

// SendAnnounce creates and notifies peer of the transfer t
func (p *peerService) SendAnnounce(t *transfer) {
	pt := p.getOrCreatePeerTransfer(t)

	pt.logger.Debug("announcing swarm")

	go func() {
		req := &transferv1.TransferPeerAnnounceRequest{
			Id:      t.id[:],
			Channel: pt.channel,
		}
		res := &transferv1.TransferPeerAnnounceResponse{}
		err := p.client.Announce(p.ctx, req, res)
		if err != nil {
			pt.logger.Debug("announce failed", zap.Error(err))
			return
		}

		if res.GetChannel() != 0 {
			p.startPeerTransfer(pt, res.GetChannel())
		}
	}()
}

// Remove cleans up the peer transfer for the removed transfer t
func (p *peerService) Remove(t *transfer) {
	p.lock.Lock()
	pt, ok := p.transfers[t.id]
	if ok {
		delete(p.transfers, t.id)
	}
	p.lock.Unlock()

	if ok {
		pt.logger.Debug("removed peer transfer")
		p.stopPeerTransfer(pt, true)
	}
}

func (p *peerService) getPeerTransfer(id ID) (*peerTransfer, bool) {
	p.lock.Lock()
	defer p.lock.Unlock()

	pt, ok := p.transfers[id]
	return pt, ok
}

func (p *peerService) getOrCreatePeerTransfer(t *transfer) *peerTransfer {
	p.lock.Lock()
	defer p.lock.Unlock()

	pt, ok := p.transfers[t.id]
	if ok {
		pt.logger.Debug("reused peer transfer")
		return pt
	}

	p.nextChannel++

	pt = &peerTransfer{
		logger: p.logger.With(
			logutil.ByteHex("id", t.id[:]),
			zap.Stringer("swarm", t.swarm.ID()),
			zap.Uint64("localChannel", p.nextChannel),
		),
		transfer: t,
		channel:  p.nextChannel,
		stop:     make(chan struct{}),
	}
	p.transfers[t.id] = pt

	pt.logger.Debug("created peer transfer")

	return pt
}

func (p *peerService) startPeerTransfer(pt *peerTransfer, peerChannel uint64) bool {
	pt.lock.Lock()
	defer pt.lock.Unlock()

	if pt.open {
		return true
	}

	pt.logger.Debug(
		"opening swarm channel",
		zap.Uint64("peerChannel", peerChannel),
	)

	err := p.runnerPeer.RunSwarm(pt.swarm, codec.Channel(pt.channel), codec.Channel(peerChannel))
	if err != nil {
		pt.logger.Debug("unable to start swarm channel", zap.Error(err))
		return false
	}
	pt.open = true

	go func() {
		select {
		case <-pt.ctx.Done():
			p.stopPeerTransfer(pt, true)
		case <-p.ctx.Done():
			p.stopPeerTransfer(pt, false)
		case <-pt.stop:
		}
	}()

	return true
}

func (p *peerService) stopPeerTransfer(pt *peerTransfer, notifyPeer bool) {
	pt.lock.Lock()
	defer pt.lock.Unlock()

	if !pt.open {
		return
	}

	pt.logger.Debug("closing swarm channel")

	p.runnerPeer.StopSwarm(pt.swarm)
	pt.open = false

	select {
	case pt.stop <- struct{}{}:
	default:
	}

	if notifyPeer {
		req := &transferv1.TransferPeerCloseRequest{Id: pt.id[:]}
		res := &transferv1.TransferPeerCloseResponse{}
		err := p.client.Close(context.Background(), req, res)
		if err != nil {
			pt.logger.Debug("unable to notify peer of channel closure", zap.Error(err))
		}
	}
}

type peerTransfer struct {
	logger  *zap.Logger
	lock    sync.Mutex
	channel uint64
	stop    chan struct{}
	open    bool
	*transfer
}
