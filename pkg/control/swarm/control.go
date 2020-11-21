package swarm

import (
	"context"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, observers *event.Observers) *Control {
	events := make(chan interface{}, 128)
	observers.Peer.Notify(events)
	observers.Network.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		observers: observers,
		events:    events,
		swarms:    map[uint64]*Swarm{},
		peers:     map[uint64]*Peer{},
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	lock      sync.Mutex
	observers *event.Observers
	events    chan interface{}
	swarms    map[uint64]*Swarm
	nextID    uint64
	peers     map[uint64]*Peer
}

// Run ...
func (t *Control) Run(ctx context.Context) {
	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			// case PeerAddEvent:
			// 	t.handlePeerAdd(event.Peer, event.Client)
			// case PeerRemoveEvent:
			// 	t.handlePeerRemove(event.Peer)
			case event.NetworkStop:
				t.handleVPNStop(e.Network)
			}
		case <-ctx.Done():
			return
		}
	}
}

// AddPeer ...
func (t *Control) AddPeer(id uint64, peer *vnic.Peer, client PeerClient) *Peer {
	p := &Peer{
		Peer:   peer,
		Client: client,
	}

	t.lock.Lock()
	t.lock.Unlock()

	t.peers[id] = p

	return p
}

// RemovePeer ...
func (t *Control) RemovePeer(id uint64) {
	t.lock.Lock()
	defer t.lock.Unlock()

	p, ok := t.peers[id]
	if !ok {
		return
	}

	_ = p

	delete(t.peers, id)
}

func (t *Control) handleVPNStop(network *pb.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

}

// Add ...
func (t *Control) Add(swarm *ppspp.Swarm) *Swarm {
	s := &Swarm{
		ID:    atomic.AddUint64(&t.nextID, 1),
		Swarm: swarm,
	}

	t.lock.Lock()
	defer t.lock.Unlock()

	t.swarms[s.ID] = s
	t.observers.Swarm.Emit(event.SwarmAdd{swarm})

	return s
}

// Remove ...
func (t *Control) Remove(p *Swarm) {
	t.lock.Lock()
	defer t.lock.Unlock()

	delete(t.swarms, p.ID)
	t.observers.Swarm.Emit(event.SwarmRemove{p.Swarm})
}

// Swarm ...
type Swarm struct {
	ID    uint64
	Swarm *ppspp.Swarm
}
