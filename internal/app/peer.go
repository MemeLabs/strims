package app

import (
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/internal/api"
	"github.com/MemeLabs/go-ppspp/internal/bootstrap"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
)

type PeerControl interface {
	Add(peer *vnic.Peer, client api.PeerClient) Peer
	Remove(p Peer)
	Get(id uint64) Peer
	List() []Peer
}

// NewPeerControl ...
func NewPeerControl(
	observers *event.Observers,
	network network.Control,
	transfer transfer.Control,
	bootstrap bootstrap.Control,
) PeerControl {
	return &peerControl{
		observers: observers,
		network:   network,
		transfer:  transfer,
		bootstrap: bootstrap,
		peers:     map[uint64]*peer{},
	}
}

// PeerControl ...
type peerControl struct {
	observers *event.Observers
	network   network.Control
	transfer  transfer.Control
	bootstrap bootstrap.Control

	lock   sync.Mutex
	nextID uint64
	peers  map[uint64]*peer
}

// Add ...
func (t *peerControl) Add(vnicPeer *vnic.Peer, client api.PeerClient) Peer {
	id := atomic.AddUint64(&t.nextID, 1)
	p := &peer{
		id:        id,
		vnic:      vnicPeer,
		client:    client,
		network:   t.network.AddPeer(id, vnicPeer, client),
		transfer:  t.transfer.AddPeer(id, vnicPeer, client),
		bootstrap: t.bootstrap.AddPeer(id, vnicPeer, client),
	}

	t.lock.Lock()
	t.peers[p.id] = p
	t.lock.Unlock()

	t.observers.EmitLocal(event.PeerAdd{ID: id, VNIC: vnicPeer})

	return p
}

// Remove ...
func (t *peerControl) Remove(p Peer) {
	t.lock.Lock()
	delete(t.peers, p.ID())
	t.lock.Unlock()

	t.network.RemovePeer(p.ID())
	t.transfer.RemovePeer(p.ID())
	t.bootstrap.RemovePeer(p.ID())

	t.observers.EmitLocal(event.PeerRemove{ID: p.ID()})
}

// Get ...
func (t *peerControl) Get(id uint64) Peer {
	t.lock.Lock()
	defer t.lock.Unlock()
	return t.peers[id]
}

// List ...
func (t *peerControl) List() []Peer {
	t.lock.Lock()
	defer t.lock.Unlock()

	peers := make([]Peer, len(t.peers))
	var n int
	for _, p := range t.peers {
		peers[n] = p
		n++
	}

	return peers
}

// Peer ...
type Peer interface {
	ID() uint64
	Client() api.PeerClient
	VNIC() *vnic.Peer
	Network() network.Peer
	Transfer() transfer.Peer
	Bootstrap() bootstrap.Peer
}

// Peer ...
type peer struct {
	id        uint64
	client    api.PeerClient
	vnic      *vnic.Peer
	network   network.Peer
	transfer  transfer.Peer
	bootstrap bootstrap.Peer
}

// ID ...
func (p *peer) ID() uint64 { return p.id }

// Client ...
func (p *peer) Client() api.PeerClient { return p.client }

// VNIC ...
func (p *peer) VNIC() *vnic.Peer { return p.vnic }

// Network ...
func (p *peer) Network() network.Peer { return p.network }

// Transfer ...
func (p *peer) Transfer() transfer.Peer { return p.transfer }

// Bootstrap ...
func (p *peer) Bootstrap() bootstrap.Peer { return p.bootstrap }
