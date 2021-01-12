package transfer

import (
	"bytes"
	"context"
	"sync"
	"time"

	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/api"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// Peer ...
type Peer struct {
	logger    *zap.Logger
	vnicPeer  *vnic.Peer
	swarmPeer *ppspp.Peer
	client    api.PeerClient

	lock   sync.Mutex
	swarms llrb.LLRB
}

// AssignPort ...
func (p *Peer) AssignPort(id []byte, peerPort uint16) (uint16, bool) {
	s, ok := p.getPeerTransfer(id)
	if !ok {
		return 0, false
	}

	p.openChannel(s, peerPort)

	return s.port, true
}

// AnnounceSwarm ...
func (p *Peer) AnnounceSwarm(swarm *ppspp.Swarm) {
	s, err := p.getOrCreatePeerTransfer(swarm)
	if err != nil {
		return
	}

	ctx, cancel := context.WithTimeout(p.vnicPeer.Context(), time.Second)
	defer cancel()

	req := &transferv1.TransferPeerAnnounceSwarmRequest{
		SwarmId: swarm.ID(),
		Port:    uint32(s.port),
	}
	res := &transferv1.TransferPeerAnnounceSwarmResponse{}
	err = p.client.Transfer().AnnounceSwarm(ctx, req, res)
	if err != nil {
		p.logger.Debug("failed", zap.Error(err))
		return
	}

	p.openChannel(s, uint16(res.GetPort()))
}

// CloseSwarm ...
func (p *Peer) CloseSwarm(swarm *ppspp.Swarm) {
	if s, ok := p.getPeerTransfer(swarm.ID()); ok {
		p.closeChannel(s)
	}
}

func (p *Peer) getPeerTransfer(id []byte) (*peerTransfer, bool) {
	p.lock.Lock()
	defer p.lock.Unlock()

	s, ok := p.swarms.Get(&peerTransfer{id: id}).(*peerTransfer)
	return s, ok
}

func (p *Peer) getOrCreatePeerTransfer(swarm *ppspp.Swarm) (*peerTransfer, error) {
	p.lock.Lock()
	defer p.lock.Unlock()

	s, ok := p.swarms.Get(&peerTransfer{id: swarm.ID()}).(*peerTransfer)
	if !ok {
		port, err := p.vnicPeer.ReservePort()
		if err != nil {
			return nil, err
		}

		s = &peerTransfer{
			id:    swarm.ID(),
			swarm: swarm,
			port:  port,
		}
		p.swarms.ReplaceOrInsert(s)
	}

	return s, nil
}

func (p *Peer) openChannel(s *peerTransfer, peerPort uint16) {
	p.lock.Lock()
	defer p.lock.Unlock()

	if s.channel != nil || peerPort == 0 {
		return
	}

	p.logger.Debug(
		"opening swarm channel",
		zap.Stringer("peer", p.vnicPeer.HostID()),
		zap.Stringer("swarm", s.swarm.ID()),
		zap.Uint16("localPort", s.port),
		zap.Uint16("peerPort", peerPort),
	)

	w := vnic.NewFrameWriter(p.vnicPeer.Link, peerPort)
	ch, err := ppspp.OpenChannel(p.logger, p.swarmPeer, s.swarm, w)
	if err != nil {
		return
	}
	s.channel = ch

	p.vnicPeer.SetHandler(s.port, func(p *vnic.Peer, f vnic.Frame) error {
		_, err := ch.HandleMessage(f.Body)
		if err != nil {
			ch.Close()
		}
		return err
	})

	go func() {
		select {
		case <-p.vnicPeer.Done():
		case <-ch.Done():
		}

		p.lock.Lock()
		defer p.lock.Unlock()

		ch.Close()
		ppspp.CloseChannel(p.swarmPeer, s.swarm)
		p.vnicPeer.RemoveHandler(s.port)

		s.channel = nil

		p.logger.Debug(
			"closed swarm channel",
			zap.Stringer("peer", p.vnicPeer.HostID()),
			zap.Stringer("swarm", s.swarm.ID()),
			zap.Uint16("localPort", s.port),
			zap.Uint16("peerPort", peerPort),
		)
	}()
}

func (p *Peer) closeChannel(s *peerTransfer) {
	p.lock.Lock()
	defer p.lock.Unlock()

	if s.channel != nil {
		s.channel.Close()
	}
}

type peerTransfer struct {
	id      []byte
	swarm   *ppspp.Swarm
	port    uint16
	channel *ppspp.ChannelReader
}

func (s *peerTransfer) Less(o llrb.Item) bool {
	if o, ok := o.(*peerTransfer); ok {
		return bytes.Compare(s.id, o.id) == -1
	}
	return !o.Less(s)
}
