package transfer

import (
	"bytes"
	"context"
	"crypto/sha256"
	"sync"
	"time"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/api"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

var _ control.TransferControl = &Control{}

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, observers *event.Observers) *Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		qosc:      vpn.VNIC().QOS().AddClass(1),
		observers: observers,
		events:    events,
		peers:     map[uint64]*Peer{},
		runner:    ppspp.NewRunner(context.Background(), logger),
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	qosc      *qos.Class
	lock      sync.Mutex
	observers *event.Observers
	events    chan interface{}
	transfers llrb.LLRB
	peers     map[uint64]*Peer
	networks  llrb.LLRB
	runner    *ppspp.Runner

	contactedHosts llrb.LLRB
}

// Run ...
func (c *Control) Run(ctx context.Context) {
	loadPeersTicker := time.NewTicker(10 * time.Second)
	defer loadPeersTicker.Stop()

	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			case event.NetworkStart:
				c.handleNetworkStart(dao.NetworkKey(e.Network))
			case event.NetworkStop:
				c.handleNetworkStop(dao.NetworkKey(e.Network))
			case event.NetworkPeerOpen:
				c.handleNetworkPeerOpen(e.PeerID, e.NetworkKey)
			case event.NetworkPeerClose:
				c.handleNetworkPeerClose(e.PeerID, e.NetworkKey)
			}
		case <-loadPeersTicker.C:
			c.loadPeers(ctx)
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
		p.Announce(i.(*transfer))
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

	// TODO: close peer transfers associated with this network?
}

func (c *Control) loadPeers(ctx context.Context) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.networks.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		c.loadNetworkPeers(ctx, i.(*network))
		return true
	})
}

func (c *Control) loadNetworkPeers(ctx context.Context, n *network) {
	node, ok := c.vpn.Node(n.key)
	if !ok {
		return
	}

	n.transfers.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		t := i.(*transfer)

		c.logger.Debug(
			"searching for peers for",
			logutil.ByteHex("swarm", t.swarm.ID()),
			logutil.ByteHex("salt", t.salt),
		)

		ctx, cancel := context.WithTimeout(ctx, 10*time.Second)
		s, err := node.PeerIndex.Search(ctx, t.swarm.ID(), t.salt)
		if err != nil {
			cancel()
			return true
		}

		go func() {
			for p := range s {
				// debug.PrintJSON(p)

				// TODO: store, sort, swap low perf hosts experimentally
				c.connectFoundHost(n, p)
			}
			cancel()
		}()
		return true
	})
}

func (c *Control) connectFoundHost(n *network, p *vpn.PeerIndexHost) {
	c.lock.Lock()
	defer c.lock.Unlock()

	if c.contactedHosts.Has(&contactedHost{p.HostID}) {
		return
	}
	c.contactedHosts.InsertNoReplace(&contactedHost{p.HostID})

	node, ok := c.vpn.Node(n.key)
	if !ok {
		return
	}

	node.PeerExchange.Connect(p.HostID)
}

type contactedHost struct {
	id kademlia.ID
}

func (f *contactedHost) Less(o llrb.Item) bool {
	if o, ok := o.(*contactedHost); ok {
		return f.id.Less(o.id)
	}
	return !o.Less(f)
}

// AddPeer ...
func (c *Control) AddPeer(id uint64, peer *vnic.Peer, client api.PeerClient) *Peer {
	ctx, close := context.WithCancel(peer.Context())
	w := vnic.NewFrameWriter(peer.Link, vnic.TransferPort, c.qosc)
	cr, rp := c.runner.RunPeer(peer.HostID().Bytes(nil), w)
	p := &Peer{
		logger:     c.logger.With(zap.Stringer("peer", peer.HostID())),
		ctx:        ctx,
		vnicPeer:   peer,
		runnerPeer: rp,
		client:     client,
	}
	peer.SetHandler(vnic.TransferPort, func(_ *vnic.Peer, f vnic.Frame) error {
		err := cr.HandleMessage(f.Body)
		if err != nil {
			close()
		}
		return err
	})

	c.lock.Lock()
	c.peers[id] = p
	c.lock.Unlock()

	return p
}

// RemovePeer ...
func (c *Control) RemovePeer(id uint64) {
	c.lock.Lock()
	p, ok := c.peers[id]
	delete(c.peers, id)
	c.lock.Unlock()

	if !ok {
		return
	}

	p.vnicPeer.RemoveHandler(vnic.TransferPort)
	p.runnerPeer.Stop()
}

// Add ...
func (c *Control) Add(swarm *ppspp.Swarm, salt []byte) []byte {
	h := sha256.New()
	h.Write(swarm.ID())
	h.Write(salt)
	id := h.Sum(nil)

	ctx, close := context.WithCancel(context.Background())

	t := &transfer{
		id:    id,
		salt:  salt,
		ctx:   ctx,
		close: close,
		swarm: swarm,
	}

	c.lock.Lock()
	c.transfers.ReplaceOrInsert(t)
	c.lock.Unlock()

	c.logger.Debug(
		"added swarm",
		logutil.ByteHex("id", t.id),
		logutil.ByteHex("swarm", swarm.ID()),
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

	c.transfers.Delete(t)
	c.networks.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		i.(*network).transfers.Delete(t)
		return true
	})

	t.close()

	for _, p := range c.peers {
		p.Remove(t)
	}

	c.logger.Debug(
		"closed swarm",
		logutil.ByteHex("id", t.id),
		logutil.ByteHex("swarm", t.swarm.ID()),
	)
}

// List ...
func (c *Control) List() []*transferv1.Transfer {
	c.lock.Lock()
	defer c.lock.Unlock()

	ts := make([]*transferv1.Transfer, c.transfers.Len(), 0)
	c.transfers.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		ts = append(ts, &transferv1.Transfer{
			Id: i.(*transfer).id,
		})
		return true
	})

	return ts
}

// Publish ...
func (c *Control) Publish(id []byte, networkKey []byte) {
	c.lock.Lock()

	t, ok := c.transfers.Get(&transfer{id: id}).(*transfer)
	if !ok {
		c.lock.Unlock()
		return
	}

	n := c.getOrInsertNetwork(networkKey)
	n.transfers.ReplaceOrInsert(t)

	for _, p := range n.peers {
		p.Announce(t)
	}

	c.lock.Unlock()

	node, ok := c.vpn.Node(networkKey)
	if !ok {
		return
	}

	err := node.PeerIndex.Publish(t.ctx, t.swarm.ID(), t.salt, 0)
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

func (t *transfer) Less(o llrb.Item) bool {
	if o, ok := o.(*transfer); ok {
		return bytes.Compare(t.id, o.id) == -1
	}
	return !o.Less(t)
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
