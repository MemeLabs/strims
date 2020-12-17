package transfer

import (
	"context"
	"log"
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
		scheduler: ppspp.NewScheduler(context.Background(), logger),
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
	scheduler *ppspp.Scheduler
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
			case event.NetworkStart:
				t.handleNetworkStart(e.Network)
			case event.NetworkStop:
				t.handleNetworkStop(e.Network)
			case event.NetworkPeerOpen:
				t.handleNetworkPeerOpen()
			case event.NetworkPeerClose:
				t.handleNetworkPeerClose()
			}
		case <-ctx.Done():
			return
		}
	}
}

func (t *Control) handleNetworkStart(network *pb.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()
}

func (t *Control) handleNetworkStop(network *pb.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

}

func (t *Control) handleNetworkPeerOpen() {
	log.Println("do something with peer network...")
}

func (t *Control) handleNetworkPeerClose() {
	log.Println("do something with peer network...")
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

// Add ...
func (t *Control) Add(swarm *ppspp.Swarm) *Swarm {
	s := &Swarm{
		ID:    atomic.AddUint64(&t.nextID, 1),
		Swarm: swarm,
	}

	t.lock.Lock()
	defer t.lock.Unlock()

	t.swarms[s.ID] = s
	t.observers.Swarm.Emit(event.SwarmAdd{Swarm: swarm})

	return s
}

// Remove ...
func (t *Control) Remove(swarmID ppspp.SwarmID) {
	t.lock.Lock()
	defer t.lock.Unlock()

	// delete(t.swarms, swarm.ID)
	// t.observers.Swarm.Emit(event.SwarmRemove{Swarm: swarm.Swarm})
}

// List ...
func (t *Control) List() []*Swarm {
	t.lock.Lock()
	defer t.lock.Unlock()

	return nil
}

// Publish ...
func (t *Control) Publish(swarmID ppspp.SwarmID, networkKey []byte) {
	client, ok := t.vpn.Client(networkKey)
	if !ok {
		return
	}

	t.lock.Lock()
	defer t.lock.Unlock()

	_ = client

	// swarm, ok := t.swarms[swarmID]
	// if !ok {
	// 	return
	// }

	// client.PeerIndex.Publish(swarm.ctx, swarm.Swarm.ID(), []byte{}, 0)
}

// Swarm ...
type Swarm struct {
	ctx      context.Context
	ID       uint64
	Swarm    *ppspp.Swarm
	Networks []*Network
}

// Publish ...
func (s *Swarm) Publish(networkKey []byte) {
	// s.Networks = append(s.Networks)
}

// Network ...
type Network struct {
}
