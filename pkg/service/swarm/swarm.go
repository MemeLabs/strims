package swarm

import (
	"bytes"
	"context"
	"log"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
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

func newSwarmPeer(peer *vnic.Peer) *swarmPeer {
	rw := vnic.NewFrameReadWriter(peer.Link, vnic.SwarmPort, peer.Link.MTU())
	peer.SetHandler(vnic.SwarmPort, rw.HandleFrame)

	return &swarmPeer{
		peer:       peer,
		swarmPeer:  ppspp.NewPeer(peer.HostID().Bytes(nil)),
		rw:         rw,
		swarmPorts: map[string]*swarmPeerSwarmItem{},
	}
}

type swarmPeer struct {
	peer           *vnic.Peer
	swarmPeer      *ppspp.Peer
	rw             *vnic.FrameReadWriter
	swarmPortsLock sync.Mutex
	swarmPorts     map[string]*swarmPeerSwarmItem
}

func (p *swarmPeer) findOrInsertSwarm(id ppspp.SwarmID) *swarmPeerSwarmItem {
	p.swarmPortsLock.Lock()
	defer p.swarmPortsLock.Unlock()

	if old, ok := p.swarmPorts[id.String()]; ok {
		return old
	}

	item := &swarmPeerSwarmItem{}
	p.swarmPorts[id.String()] = item
	return item
}

func (p *swarmPeer) SetLocalSwarmPort(id ppspp.SwarmID, port uint16) {
	p.findOrInsertSwarm(id).SetLocalPort(port)
}

func (p *swarmPeer) SetRemoteSwarmPort(id ppspp.SwarmID, port uint16) {
	p.findOrInsertSwarm(id).SetRemotePort(port)
}

func (p *swarmPeer) SwarmPorts(id ppspp.SwarmID) (uint16, uint16, bool) {
	return p.findOrInsertSwarm(id).Ports()
}

func newSwarmSwarm(logger *zap.Logger, swarm *ppspp.Swarm) *swarmSwarm {
	return &swarmSwarm{
		logger:       logger,
		swarm:        swarm,
		peerChannels: map[*swarmPeer]*ppspp.ChannelReader{},
	}
}

type swarmSwarm struct {
	logger       *zap.Logger
	swarm        *ppspp.Swarm
	peerChannels map[*swarmPeer]*ppspp.ChannelReader
}

// TODO: prevent duplicate channels...
func (s *swarmSwarm) TryOpenChannel(p *swarmPeer) {
	localPort, remotePort, ok := p.SwarmPorts(s.swarm.ID())
	if !ok {
		return
	}

	go func() {
		s.logger.Debug(
			"opening swarm channel",
			zap.Stringer("peer", p.peer.HostID()),
			zap.Stringer("swarm", s.swarm.ID()),
			zap.Uint16("localPort", localPort),
			zap.Uint16("remotePort", remotePort),
		)

		w := vnic.NewFrameWriter(p.peer.Link, remotePort, p.peer.Link.MTU())
		ch, err := ppspp.OpenChannel(s.logger, p.swarmPeer, s.swarm, w)
		if err != nil {
			return
		}
		s.peerChannels[p] = ch

		p.peer.SetHandler(localPort, func(p *vnic.Peer, f vnic.Frame) error {
			_, err := ch.HandleMessage(f.Body)
			if err != nil {
				ch.Close()
			}
			return err
		})

		select {
		case <-p.peer.Done():
		case <-ch.Done():
		}

		ch.Close()
		ppspp.CloseChannel(p.swarmPeer, s.swarm)
		p.peer.RemoveHandler(localPort)

		s.logger.Debug(
			"closed swarm channel",
			zap.Stringer("peer", p.peer.HostID()),
			zap.Stringer("swarm", s.swarm.ID()),
			zap.Uint16("localPort", localPort),
			zap.Uint16("remotePort", remotePort),
		)
	}()
}

func (s *swarmSwarm) TryCloseChannel(p *swarmPeer) {
	if ch, ok := s.peerChannels[p]; ok {
		ch.Close()
		delete(s.peerChannels, p)
	}
}

func (s *swarmSwarm) Close() {
	for _, ch := range s.peerChannels {
		ch.Close()
	}
}

// SwarmNetwork ...
type SwarmNetwork interface {
	OpenSwarm(swarm *ppspp.Swarm)
	CloseSwarm(id ppspp.SwarmID)
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
	return bytes.Compare(t.network.Key(), os.network.Key()) == -1
}

func (t *swarmNetwork) addPeer(p *swarmPeer) {
	t.peers.Store(p.peer, p)

	t.swarms.Range(func(_, si interface{}) bool {
		if err := t.sendOpen(si.(*swarmSwarm), p); err != nil {
			log.Println(err)
		}
		return true
	})
	p.rw.Flush()
}

func (t *swarmNetwork) OpenSwarm(swarm *ppspp.Swarm) {
	t.logger.Debug(
		"opening swarm",
		zap.Stringer("swarm", swarm.ID()),
		logutil.ByteHex("network", t.network.Key()),
	)

	si, ok := t.activeSwarms.Load(swarm.ID().String())
	if !ok {
		si, _ = t.activeSwarms.LoadOrStore(swarm.ID().String(), newSwarmSwarm(t.logger, swarm))
	}
	s := si.(*swarmSwarm)
	t.swarms.Store(swarm.ID().String(), s)

	t.peers.Range(func(_, pi interface{}) bool {
		if err := t.sendOpen(s, pi.(*swarmPeer)); err != nil {
			log.Println(err)
		}
		pi.(*swarmPeer).rw.Flush()
		return true
	})
}

func (t *swarmNetwork) sendOpen(s *swarmSwarm, p *swarmPeer) error {
	port, err := p.peer.ReservePort()
	if err != nil {
		return err
	}
	p.SetLocalSwarmPort(s.swarm.ID(), port)
	s.TryOpenChannel(p)

	t.logger.Debug(
		"announcing swarm to peer",
		zap.Stringer("peer", p.peer.HostID()),
		zap.Stringer("swarm", s.swarm.ID()),
		zap.Uint16("port", port),
	)

	return protoutil.WriteStream(p.rw, &pb.SwarmThingMessage{
		Body: &pb.SwarmThingMessage_Open_{
			Open: &pb.SwarmThingMessage_Open{
				SwarmId: s.swarm.ID().Binary(),
				Port:    uint32(port),
			},
		},
	})
}

func (t *swarmNetwork) CloseSwarm(id ppspp.SwarmID) {
	si, ok := t.activeSwarms.Load(id.String())
	if !ok {
		return
	}

	t.logger.Debug(
		"closing swarm",
		zap.Stringer("swarm", id),
		logutil.ByteHex("network", t.network.Key()),
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
		if err := protoutil.WriteStream(rw, msg); err != nil {
			log.Println(err)
		}
		rw.Flush()
		return true
	})

	si.(*swarmSwarm).Close()
	t.activeSwarms.Delete(id.String())
}

func newSwarmController(logger *zap.Logger, h *vnic.Host, n *vpn.Host) *swarmController {
	t := &swarmController{
		logger:    logger,
		scheduler: ppspp.NewScheduler(context.TODO(), logger),
	}

	go t.do(h, n)

	return t
}

type swarmController struct {
	logger       *zap.Logger
	peers        sync.Map
	networks     sync.Map
	activeSwarms sync.Map
	scheduler    *ppspp.Scheduler
}

func (t *swarmController) do(h *vnic.Host, n *vpn.Host) {
	peers := make(chan *vnic.Peer, 16)
	h.AddPeerHandler(func(p *vnic.Peer) { peers <- p })

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

func (t *swarmController) addPeerToNetwork(peer *vnic.Peer, network *vpn.Network) {
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

func (t *swarmController) handlePeer(peer *vnic.Peer) {
	p := newSwarmPeer(peer)
	t.peers.Store(peer, p)

	t.scheduler.AddPeer(context.TODO(), p.swarmPeer)

	go func() {
		for {
			msg := new(pb.SwarmThingMessage)
			if err := protoutil.ReadStream(p.rw, msg); err != nil {
				break
			}

			switch b := msg.Body.(type) {
			case *pb.SwarmThingMessage_Open_:
				id := ppspp.NewSwarmID(b.Open.SwarmId)
				p.SetRemoteSwarmPort(id, uint16(b.Open.Port))

				if si, ok := t.activeSwarms.Load(id.String()); ok {
					si.(*swarmSwarm).TryOpenChannel(p)
				}
			case *pb.SwarmThingMessage_Close_:
				id := ppspp.NewSwarmID(b.Close.SwarmId)
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
