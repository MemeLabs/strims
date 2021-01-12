package transfer

import (
	"bytes"
	"context"
	"crypto/sha256"
	"sync"

	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, observers *event.Observers) *Control {
	events := make(chan interface{}, 128)
	observers.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		observers: observers,
		events:    events,
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
	transfers llrb.LLRB
	peers     map[uint64]*Peer
	networks  llrb.LLRB
	scheduler *ppspp.Scheduler
}

// Run ...
func (c *Control) Run(ctx context.Context) {
	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			// case PeerAddEvent:
			// 	t.handlePeerAdd(event.Peer, event.Client)
			// case PeerRemoveEvent:
			// 	t.handlePeerRemove(event.Peer)
			case event.NetworkStart:
				c.handleNetworkStart(dao.NetworkKey(e.Network))
			case event.NetworkStop:
				c.handleNetworkStop(dao.NetworkKey(e.Network))
			case event.NetworkPeerOpen:
				c.handleNetworkPeerOpen(e.PeerID, e.NetworkKey)
			case event.NetworkPeerClose:
				c.handleNetworkPeerClose(e.PeerID, e.NetworkKey)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (c *Control) handleNetworkStart(networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.getOrInsertNetwork(networkKey)
}

func (c *Control) handleNetworkStop(networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.networks.Delete(&network{key: networkKey})
}

func (c *Control) handleNetworkPeerOpen(peerID uint64, networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	n := c.getOrInsertNetwork(networkKey)

	p, ok := c.peers[peerID]
	if !ok {
		return
	}

	n.peers[peerID] = p

	n.transfers.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		p.AnnounceSwarm(i.(*transfer).swarm)
		return true
	})
}

func (c *Control) handleNetworkPeerClose(peerID uint64, networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	n, ok := c.networks.Get(&network{key: networkKey}).(*network)
	if !ok {
		return
	}

	delete(n.peers, peerID)
}

// AddPeer ...
func (c *Control) AddPeer(id uint64, peer *vnic.Peer, client api.PeerClient) *Peer {
	p := &Peer{
		logger:    c.logger,
		vnicPeer:  peer,
		swarmPeer: ppspp.NewPeer(peer.HostID().Bytes(nil)),
		client:    client,
	}

	c.lock.Lock()
	c.lock.Unlock()

	c.peers[id] = p
	c.scheduler.AddPeer(peer.Context(), p.swarmPeer)

	return p
}

// RemovePeer ...
func (c *Control) RemovePeer(id uint64) {
	c.lock.Lock()
	defer c.lock.Unlock()

	p, ok := c.peers[id]
	if !ok {
		return
	}

	delete(c.peers, id)
	c.scheduler.RemovePeer(p.swarmPeer)
}

// Add ...
func (c *Control) Add(swarm *ppspp.Swarm, salt []byte) []byte {
	c.lock.Lock()
	defer c.lock.Unlock()

	ctx, close := context.WithCancel(context.Background())

	h := sha256.New()
	h.Write(swarm.ID())
	h.Write(salt)
	id := h.Sum(nil)

	t := &transfer{
		id:    id,
		salt:  salt,
		ctx:   ctx,
		close: close,
		swarm: swarm,
	}
	c.transfers.ReplaceOrInsert(t)

	c.logger.Debug(
		"added swarm",
		logutil.ByteHex("id", swarm.ID()),
	)

	return id
}

// Remove ...
func (c *Control) Remove(id []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	t, ok := c.transfers.Get(&transfer{id: id}).(*transfer)
	if !ok {
		return
	}

	t.swarm.Close()

	c.logger.Debug(
		"closed swarm",
		logutil.ByteHex("id", t.swarm.ID()),
	)

	t.close()
}

// List ...
func (c *Control) List() []*transferv1.Transfer {
	c.lock.Lock()
	defer c.lock.Unlock()

	ts := make([]*transferv1.Transfer, c.transfers.Len(), 0)
	c.transfers.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		ts = append(ts, &transferv1.Transfer{
			Id: i.(*transfer).swarm.ID(),
		})
		return true
	})

	return ts
}

// Publish ...
func (c *Control) Publish(id []byte, networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	n := c.getOrInsertNetwork(networkKey)

	t, ok := c.transfers.Get(&transfer{id: id}).(*transfer)
	if !ok {
		return
	}

	n.transfers.ReplaceOrInsert(t)

	for _, p := range n.peers {
		p.AnnounceSwarm(t.swarm)
	}

	node, ok := c.vpn.Node(networkKey)
	if !ok {
		return
	}

	err := node.PeerIndex.Publish(t.ctx, t.id, t.salt, 0)
	if err != nil {
		return
	}
}

func (c *Control) getOrInsertNetwork(networkKey []byte) *network {
	n, ok := c.networks.Get(&network{key: networkKey}).(*network)
	if !ok {
		n = &network{
			key:   networkKey,
			peers: map[uint64]*Peer{},
		}
		c.networks.ReplaceOrInsert(n)
	}
	return n
}

// transfer ...
type transfer struct {
	id    []byte
	salt  []byte
	ctx   context.Context
	close context.CancelFunc
	swarm *ppspp.Swarm
}

func (n *transfer) Less(o llrb.Item) bool {
	if o, ok := o.(*transfer); ok {
		return bytes.Compare(n.id, o.id) == -1
	}
	return !o.Less(n)
}

// network ...
type network struct {
	key       []byte
	peers     map[uint64]*Peer
	transfers llrb.LLRB
}

func (n *network) Less(o llrb.Item) bool {
	if o, ok := o.(*network); ok {
		return bytes.Compare(n.key, o.key) == -1
	}
	return !o.Less(n)
}
