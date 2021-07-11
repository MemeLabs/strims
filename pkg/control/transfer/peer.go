package transfer

import (
	"bytes"
	"context"
	"sync"

	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/api"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

var noopCancelFunc = func() {}

// Peer ...
type Peer struct {
	logger     *zap.Logger
	ctx        context.Context
	vnicPeer   *vnic.Peer
	runnerPeer *ppspp.RunnerPeer
	client     api.PeerClient

	lock        sync.Mutex
	transfers   llrb.LLRB
	nextChannel uint64
}

// AssignPort ...
func (p *Peer) AssignPort(id []byte, peerChannel uint64) (uint64, bool) {
	pt, ok := p.getPeerTransfer(id)
	if !ok {
		return 0, false
	}

	p.openChannel(pt, peerChannel)

	return pt.channel, true
}

// Announce ...
func (p *Peer) Announce(t *transfer) {
	pt := p.getOrCreatePeerTransfer(t)

	p.logger.Debug(
		"announcing swarm",
		logutil.ByteHex("id", pt.id),
		zap.Stringer("swarm", pt.swarm.ID()),
		zap.Uint64("localChannel", pt.channel),
	)

	go func() {
		req := &transferv1.TransferPeerAnnounceRequest{
			Id:      t.id,
			Channel: pt.channel,
		}
		res := &transferv1.TransferPeerAnnounceResponse{}
		err := p.client.Transfer().Announce(p.ctx, req, res)
		if err != nil {
			p.logger.Debug("announce failed", zap.Error(err))
			return
		}

		p.openChannel(pt, res.GetChannel())
	}()
}

// Close ...
func (p *Peer) Close(id []byte) {
	if pt, ok := p.getPeerTransfer(id); ok {
		p.lock.Lock()
		defer p.lock.Unlock()
		pt.close()
	}
}

// Remove ...
func (p *Peer) Remove(t *transfer) {
	p.lock.Lock()
	defer p.lock.Unlock()

	p.logger.Debug("removing pt", logutil.ByteHex("id", t.id))
	pt, ok := p.transfers.Delete(&peerTransfer{transfer: t}).(*peerTransfer)
	if ok {
		pt.close()
	}
}

func (p *Peer) getPeerTransfer(id []byte) (*peerTransfer, bool) {
	p.lock.Lock()
	defer p.lock.Unlock()

	pt, ok := p.transfers.Get(&peerTransfer{transfer: &transfer{id: id}}).(*peerTransfer)
	return pt, ok
}

func (p *Peer) getOrCreatePeerTransfer(t *transfer) *peerTransfer {
	p.lock.Lock()
	defer p.lock.Unlock()

	pt, ok := p.transfers.Get(&peerTransfer{transfer: t}).(*peerTransfer)
	if !ok {
		p.logger.Debug("new pt", logutil.ByteHex("id", t.id))
		p.nextChannel++

		pt = &peerTransfer{
			transfer: t,
			close:    noopCancelFunc,
			channel:  p.nextChannel,
		}
		p.transfers.ReplaceOrInsert(pt)
	} else {
		p.logger.Debug("reusing pt", logutil.ByteHex("id", t.id))
	}

	return pt
}

func (p *Peer) openChannel(pt *peerTransfer, peerChannel uint64) {
	p.lock.Lock()
	defer p.lock.Unlock()

	if pt.open || peerChannel == 0 {
		return
	}

	logger := p.logger.With(
		logutil.ByteHex("id", pt.id),
		zap.Stringer("swarm", pt.swarm.ID()),
		zap.Uint64("localChannel", pt.channel),
		zap.Uint64("peerChannel", peerChannel),
	)
	logger.Debug("opening swarm channel")

	err := p.runnerPeer.RunSwarm(pt.swarm, codec.Channel(pt.channel), codec.Channel(peerChannel))
	if err != nil {
		return
	}

	ctx, close := context.WithCancel(pt.ctx)
	pt.close = close
	pt.open = true

	go func() {
		select {
		case <-p.ctx.Done():
		case <-ctx.Done():
		}

		p.runnerPeer.StopSwarm(pt.swarm)

		if !p.vnicPeer.Closed() {
			logger.Debug("sending peer channel close")
			req := &transferv1.TransferPeerCloseRequest{
				Id: pt.id,
			}
			res := &transferv1.TransferPeerCloseResponse{}
			err = p.client.Transfer().Close(context.Background(), req, res)
			if err != nil {
				logger.Debug("closing peer channel failed", zap.Error(err))
			}
		}

		logger.Debug("closed swarm channel")

		p.lock.Lock()
		defer p.lock.Unlock()
		pt.open = false
	}()
}

type peerTransfer struct {
	*transfer
	close   context.CancelFunc
	channel uint64
	open    bool
}

func (t *peerTransfer) Less(o llrb.Item) bool {
	return bytes.Compare(t.id, o.(*peerTransfer).id) == -1
}
