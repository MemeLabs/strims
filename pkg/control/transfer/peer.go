package transfer

import (
	"bytes"
	"context"
	"fmt"
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

	p.logger.Debug(
		"assigning port",
		logutil.ByteHex("id", pt.id),
		zap.Stringer("swarm", pt.swarm.ID()),
		zap.Uint64("localChannel", pt.channel),
		zap.Uint64("peerChannel", peerChannel),
	)

	pt.commands <- peerTransferCommand{peerTransferStart, peerChannel}

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

		pt.commands <- peerTransferCommand{peerTransferStart, res.GetChannel()}
	}()
}

// Close ...
func (p *Peer) Close(id []byte) {
	if pt, ok := p.getPeerTransfer(id); ok {
		pt.commands <- peerTransferCommand{op: peerTransferStop}
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
	if ok {
		return pt
	}

	p.nextChannel++

	logger := p.logger.With(
		logutil.ByteHex("id", t.id),
		zap.Stringer("swarm", t.swarm.ID()),
		zap.Uint64("localChannel", p.nextChannel),
	)

	logger.Debug("creating peer transfer")

	pt = &peerTransfer{
		transfer: t,
		channel:  p.nextChannel,
		commands: make(chan peerTransferCommand, 8),
	}
	p.transfers.ReplaceOrInsert(pt)

	go func() {
		err := p.runPeerTransfer(logger, pt)
		logger.Debug("peer transfer closed", zap.Error(err))
	}()

	return pt
}

func (p *Peer) runPeerTransfer(logger *zap.Logger, pt *peerTransfer) error {
	for {
		select {
		case cmd := <-pt.commands:
			switch cmd.op {
			case peerTransferStart:
				if pt.open || cmd.peerChannel == 0 {
					continue
				}

				logger.Debug(
					"opening swarm channel",
					zap.Uint64("peerChannel", cmd.peerChannel),
				)

				err := p.runnerPeer.RunSwarm(pt.swarm, codec.Channel(pt.channel), codec.Channel(cmd.peerChannel))
				if err != nil {
					return fmt.Errorf("unable to start swarm channel: %w", err)
				}
				pt.open = true
			case peerTransferStop:
				logger.Debug("closing swarm channel")

				p.runnerPeer.StopSwarm(pt.swarm)
				pt.open = false

				req := &transferv1.TransferPeerCloseRequest{Id: pt.id}
				res := &transferv1.TransferPeerCloseResponse{}
				err := p.client.Transfer().Close(context.Background(), req, res)
				if err != nil {
					return fmt.Errorf("unable to notify peer of channel closure: %w", err)
				}
			}
		case <-pt.ctx.Done():
			return fmt.Errorf("transfer closed: %w", pt.ctx.Err())
		case <-p.ctx.Done():
			return fmt.Errorf("peer closed: %w", pt.ctx.Err())
		}
	}
}

type peerTransferOp int

const (
	peerTransferStart peerTransferOp = iota
	peerTransferStop
	peerTransferShutdown
)

type peerTransferCommand struct {
	op          peerTransferOp
	peerChannel uint64
}

type peerTransfer struct {
	*transfer
	commands chan peerTransferCommand
	channel  uint64
	open     bool
}

func (t *peerTransfer) Less(o llrb.Item) bool {
	return bytes.Compare(t.id, o.(*peerTransfer).id) == -1
}
