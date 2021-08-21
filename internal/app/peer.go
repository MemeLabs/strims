package app

import (
	"sync"
	"sync/atomic"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/api"
	"github.com/MemeLabs/go-ppspp/internal/bootstrap"
	"github.com/MemeLabs/go-ppspp/internal/ca"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"

	"go.uber.org/zap"
)

// NewPeerControl ...
func NewPeerControl(logger *zap.Logger, observers *event.Observers, ca *ca.Control, network *network.Control, transfer *transfer.Control, bootstrap *bootstrap.Control) *PeerControl {
	return &PeerControl{
		logger:    logger,
		observers: observers,
		ca:        ca,
		network:   network,
		transfer:  transfer,
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
	transfer  *transfer.Control
	bootstrap *bootstrap.Control

	lock   sync.Mutex
	nextID uint64
	peers  map[uint64]*Peer
}

// Add ...
func (t *PeerControl) Add(peer *vnic.Peer, client api.PeerClient) control.Peer {
	id := atomic.AddUint64(&t.nextID, 1)
	p := &Peer{
		id:        id,
		vnic:      peer,
		client:    client,
		network:   t.network.AddPeer(id, peer, client),
		transfer:  t.transfer.AddPeer(id, peer, client),
		bootstrap: t.bootstrap.AddPeer(id, peer, client),
	}

	t.lock.Lock()
	t.peers[p.id] = p
	t.lock.Unlock()

	t.observers.EmitLocal(event.PeerAdd{ID: id, VNIC: peer})

	return p
}

// Remove ...
func (t *PeerControl) Remove(p control.Peer) {
	t.lock.Lock()
	delete(t.peers, p.ID())
	t.lock.Unlock()

	t.network.RemovePeer(p.ID())
	t.transfer.RemovePeer(p.ID())
	t.bootstrap.RemovePeer(p.ID())

	t.observers.EmitLocal(event.PeerRemove{ID: p.ID()})
}

// Get ...
func (t *PeerControl) Get(id uint64) control.Peer {
	t.lock.Lock()
	defer t.lock.Unlock()
	return t.peers[id]
}

// List ...
func (t *PeerControl) List() []control.Peer {
	t.lock.Lock()
	defer t.lock.Unlock()

	peers := make([]control.Peer, len(t.peers))
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
	client    api.PeerClient
	vnic      *vnic.Peer
	network   *network.Peer
	transfer  *transfer.Peer
	bootstrap *bootstrap.Peer
}

// ID ...
func (p *Peer) ID() uint64 { return p.id }

// Client ...
func (p *Peer) Client() api.PeerClient { return p.client }

// VNIC ...
func (p *Peer) VNIC() *vnic.Peer { return p.vnic }

// Network ...
func (p *Peer) Network() control.NetworkPeerControl { return p.network }

// Transfer ...
func (p *Peer) Transfer() control.TransferPeerControl { return p.transfer }

// Bootstrap ...
func (p *Peer) Bootstrap() control.BootstrapPeerControl { return p.bootstrap }
