package app

import (
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/control/ca"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/swarm"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"

	"go.uber.org/zap"
)

// NewPeerControl ...
func NewPeerControl(logger *zap.Logger, observers *event.Observers, ca *ca.Control, network *network.Control, swarm *swarm.Control, bootstrap *bootstrap.Control) *PeerControl {
	events := make(chan interface{}, 128)
	observers.VPN.Notify(events)

	return &PeerControl{
		logger:    logger,
		observers: observers,
		ca:        ca,
		network:   network,
		swarm:     swarm,
		bootstrap: bootstrap,
		peers:     map[uint64]*Peer{},
	}
}

// PeerControl ...
type PeerControl struct {
	logger    *zap.Logger
	observers *event.Observers
	ca        *ca.Control
	network   *network.Control
	swarm     *swarm.Control
	bootstrap *bootstrap.Control

	lock   sync.Mutex
	nextID uint64
	peers  map[uint64]*Peer
}

// PeerClient ..
type PeerClient interface {
	Swarm() *api.SwarmPeerClient
	Bootstrap() *api.BootstrapPeerClient
	CA() *api.CAPeerClient
	Network() *api.NetworkPeerClient
}

// Add ...
func (t *PeerControl) Add(peer *vnic.Peer, client PeerClient) *Peer {
	id := atomic.AddUint64(&t.nextID, 1)
	p := &Peer{
		id:        id,
		vnic:      peer,
		client:    client,
		network:   t.network.AddPeer(id, peer, client),
		swarm:     t.swarm.AddPeer(id, peer, client),
		bootstrap: t.bootstrap.AddPeer(id, peer, client),
	}

	t.lock.Lock()
	t.peers[p.id] = p
	t.lock.Unlock()

	t.observers.Peer.Emit(event.PeerAdd{ID: id, VNIC: peer})

	return p
}

// Remove ...
func (t *PeerControl) Remove(p *Peer) {
	t.lock.Lock()
	delete(t.peers, p.id)
	t.lock.Unlock()

	t.network.RemovePeer(p.id)
	t.swarm.RemovePeer(p.id)
	t.bootstrap.RemovePeer(p.id)

	t.observers.Peer.Emit(event.PeerRemove{ID: p.id})
}

// Get ...
func (t *PeerControl) Get(id uint64) *Peer {
	t.lock.Lock()
	defer t.lock.Unlock()
	return t.peers[id]
}

// List ...
func (t *PeerControl) List() []*Peer {
	t.lock.Lock()
	defer t.lock.Unlock()

	peers := make([]*Peer, len(t.peers))
	var n int
	for _, p := range t.peers {
		peers[n] = p
		n++
	}

	return peers
}

// Peer ...
type Peer struct {
	id        uint64
	client    PeerClient
	vnic      *vnic.Peer
	network   *network.Peer
	swarm     *swarm.Peer
	bootstrap *bootstrap.Peer
}

// ID ...
func (p *Peer) ID() uint64 { return p.id }

// Client ...
func (p *Peer) Client() PeerClient { return p.client }

// VNIC ...
func (p *Peer) VNIC() *vnic.Peer { return p.vnic }

// Network ...
func (p *Peer) Network() *network.Peer { return p.network }

// Swarm ...
func (p *Peer) Swarm() *swarm.Peer { return p.swarm }

// Bootstrap ...
func (p *Peer) Bootstrap() *bootstrap.Peer { return p.bootstrap }
