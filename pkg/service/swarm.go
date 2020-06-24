package service

import (
	"bytes"
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

type swarmPeerSwarmItem struct {
	lock       sync.Mutex
	localPort  uint16
	remotePort uint16
}

func (s *swarmPeerSwarmItem) SetLocalPort(p uint16) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.localPort = p
}

func (s *swarmPeerSwarmItem) SetRemotePort(p uint16) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.remotePort = p
}

func (s *swarmPeerSwarmItem) Ports() (uint16, uint16, bool) {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.localPort, s.remotePort, s.localPort != 0 && s.remotePort != 0
}

func newSwarmPeer(peer *vpn.Peer) *swarmPeer {
	rw := vpn.NewFrameReadWriter(peer.Link, vpn.SwarmPort, peer.Link.MTU())
	peer.SetHandler(vpn.SwarmPort, rw.HandleFrame)

	return &swarmPeer{
		peer:       peer,
		swarmPeer:  encoding.NewPeer(),
		rw:         rw,
		swarmPorts: map[string]*swarmPeerSwarmItem{},
	}
}

type swarmPeer struct {
	peer           *vpn.Peer
	swarmPeer      *encoding.Peer
	rw             *vpn.FrameReadWriter
	swarmPortsLock sync.Mutex
	swarmPorts     map[string]*swarmPeerSwarmItem
}

func (p *swarmPeer) findOrInsertSwarm(id encoding.SwarmID) *swarmPeerSwarmItem {
	p.swarmPortsLock.Lock()
	defer p.swarmPortsLock.Unlock()

	if old, ok := p.swarmPorts[id.String()]; ok {
		return old
	}

	item := &swarmPeerSwarmItem{}
	p.swarmPorts[id.String()] = item
	return item
}

func (p *swarmPeer) SetLocalSwarmPort(id encoding.SwarmID, port uint16) {
	p.findOrInsertSwarm(id).SetLocalPort(port)
}

func (p *swarmPeer) SetRemoteSwarmPort(id encoding.SwarmID, port uint16) {
	p.findOrInsertSwarm(id).SetRemotePort(port)
}

func (p *swarmPeer) SwarmPorts(id encoding.SwarmID) (uint16, uint16, bool) {
	return p.findOrInsertSwarm(id).Ports()
}

func newSwarmSwarm(logger *zap.Logger, swarm *encoding.Swarm) *swarmSwarm {
	return &swarmSwarm{
		logger: logger,
		swarm:  swarm,
	}
}

type swarmSwarm struct {
	logger *zap.Logger
	swarm  *encoding.Swarm
}

// TODO: prevent duplicate channels...
func (s *swarmSwarm) TryOpenChannel(p *swarmPeer) {
	localPort, remotePort, ok := p.SwarmPorts(s.swarm.ID)
	if !ok {
		return
	}

	go func() {
		s.logger.Debug(
			"opening swarm channel",
			logutil.ByteHex("peer", p.peer.Certificate.Key),
			zap.Stringer("swarm", s.swarm.ID),
			zap.Uint16("localPort", localPort),
			zap.Uint16("remotePort", remotePort),
		)

		w := vpn.NewFrameWriter(p.peer.Link, remotePort, p.peer.Link.MTU())
		ch := s.swarm.ReadChannel(p.swarmPeer, w)
		p.peer.SetHandler(localPort, func(p *vpn.Peer, f vpn.Frame) error {
			_, err := ch.Write(f.Body)
			if err != nil {
				ch.Close()
			}
			return err
		})

		<-ch.Done()
		p.peer.RemoveHandler(localPort)

		s.logger.Debug(
			"closed swarm channel",
			logutil.ByteHex("peer", p.peer.Certificate.Key),
			zap.Stringer("swarm", s.swarm.ID),
			zap.Uint16("localPort", localPort),
			zap.Uint16("remotePort", remotePort),
		)
	}()
}

func (s *swarmSwarm) TryCloseChannel(p *swarmPeer) {

}

// SwarmNetwork ...
type SwarmNetwork interface {
	OpenSwarm(swarm *encoding.Swarm)
	CloseSwarm(id encoding.SwarmID)
}

func newSwarmNetwork(logger *zap.Logger, n *vpn.Network, s *sync.Map) *swarmNetwork {
	return &swarmNetwork{
		logger:       logger,
		network:      n,
		activeSwarms: s,
	}
}

type swarmNetwork struct {
	logger       *zap.Logger
	network      *vpn.Network
	activeSwarms *sync.Map
	swarms       sync.Map
	peers        sync.Map
}

func (t *swarmNetwork) Less(o llrb.Item) bool {
	os, ok := o.(*swarmNetwork)
	if !ok {
		return !o.Less(t)
	}
	return bytes.Compare(t.network.CAKey(), os.network.CAKey()) == -1
}

func (t *swarmNetwork) addPeer(p *swarmPeer) {
	t.peers.Store(p.peer, p)

	t.swarms.Range(func(_, si interface{}) bool {
		t.sendOpen(si.(*swarmSwarm), p)
		return true
	})
	p.rw.Flush()
}

func (t *swarmNetwork) OpenSwarm(swarm *encoding.Swarm) {
	t.logger.Debug(
		"opening swarm",
		zap.Stringer("swarm", swarm.ID),
		logutil.ByteHex("network", t.network.CAKey()),
	)

	si, ok := t.activeSwarms.Load(swarm.ID.String())
	if !ok {
		si, _ = t.activeSwarms.LoadOrStore(swarm.ID.String(), newSwarmSwarm(t.logger, swarm))
	}
	s := si.(*swarmSwarm)
	t.swarms.Store(swarm.ID.String(), s)

	t.peers.Range(func(_, pi interface{}) bool {
		t.sendOpen(s, pi.(*swarmPeer))
		pi.(*swarmPeer).rw.Flush()
		return true
	})
}

func (t *swarmNetwork) sendOpen(s *swarmSwarm, p *swarmPeer) error {
	port, err := p.peer.ReservePort()
	if err != nil {
		return err
	}
	p.SetLocalSwarmPort(s.swarm.ID, port)
	s.TryOpenChannel(p)

	t.logger.Debug(
		"announcing swarm to peer",
		logutil.ByteHex("peer", p.peer.Certificate.Key),
		zap.Stringer("swarm", s.swarm.ID),
		zap.Uint16("port", port),
	)

	return vpn.WriteProtoStream(p.rw, &pb.SwarmThingMessage{
		Body: &pb.SwarmThingMessage_Open_{
			Open: &pb.SwarmThingMessage_Open{
				SwarmId: s.swarm.ID.Binary(),
				Port:    uint32(port),
			},
		},
	})
}

func (t *swarmNetwork) CloseSwarm(id encoding.SwarmID) {
	t.logger.Debug(
		"closing swarm",
		zap.Stringer("swarm", id),
		logutil.ByteHex("network", t.network.CAKey()),
	)

	msg := &pb.SwarmThingMessage{
		Body: &pb.SwarmThingMessage_Close_{
			Close: &pb.SwarmThingMessage_Close{
				SwarmId: id.Binary(),
			},
		},
	}

	t.peers.Range(func(_, value interface{}) bool {
		rw := value.(*swarmPeer).rw
		vpn.WriteProtoStream(rw, msg)
		rw.Flush()
		return true
	})

	t.activeSwarms.Delete(id.String())
}

func newSwarmController(logger *zap.Logger, h *vpn.Host, n *vpn.Networks) *swarmController {
	t := &swarmController{
		logger:    logger,
		scheduler: encoding.NewScheduler(context.TODO(), logger),
	}

	go t.do(h, n)

	return t
}

type swarmController struct {
	logger       *zap.Logger
	peers        sync.Map
	networks     sync.Map
	activeSwarms sync.Map
	scheduler    *encoding.Scheduler
}

func (t *swarmController) do(h *vpn.Host, n *vpn.Networks) {
	peers := make(chan *vpn.Peer, 16)
	h.AddPeerHandler(func(p *vpn.Peer) { peers <- p })

	peerNetworks := make(chan vpn.PeerNetwork, 16)
	n.NotifyPeerNetwork(peerNetworks)

	for {
		select {
		case peer := <-peers:
			t.handlePeer(peer)
		case pn := <-peerNetworks:
			t.addPeerToNetwork(pn.Peer, pn.Network)
		}
	}
}

func (t *swarmController) addPeerToNetwork(peer *vpn.Peer, network *vpn.Network) {
	pi, ok := t.peers.Load(peer)
	if !ok {
		return
	}
	ni, ok := t.networks.Load(network)
	if !ok {
		return
	}
	ni.(*swarmNetwork).addPeer(pi.(*swarmPeer))
}

func (t *swarmController) handlePeer(peer *vpn.Peer) {
	p := newSwarmPeer(peer)
	t.peers.Store(peer, p)

	t.scheduler.AddPeer(context.TODO(), p.swarmPeer)

	go func() {
		for {
			msg := new(pb.SwarmThingMessage)
			if err := vpn.ReadProtoStream(p.rw, msg); err != nil {
				break
			}

			switch b := msg.Body.(type) {
			case *pb.SwarmThingMessage_Open_:
				id := encoding.NewSwarmID(b.Open.SwarmId)
				p.SetRemoteSwarmPort(id, uint16(b.Open.Port))

				if si, ok := t.activeSwarms.Load(id.String()); ok {
					si.(*swarmSwarm).TryOpenChannel(p)
				}
			case *pb.SwarmThingMessage_Close_:
				id := encoding.NewSwarmID(b.Close.SwarmId)
				p.SetRemoteSwarmPort(id, 0)

				if si, ok := t.activeSwarms.Load(id.String()); ok {
					si.(*swarmSwarm).TryCloseChannel(p)
				}
			}
		}
	}()

	go func() {
		<-peer.Done()
		t.peers.Delete(peer)
		p.rw.Close()
	}()
}

func (t *swarmController) AddNetwork(network *vpn.Network) *swarmNetwork {
	n := newSwarmNetwork(t.logger, network, &t.activeSwarms)
	t.networks.Store(network, n)
	return n
}
