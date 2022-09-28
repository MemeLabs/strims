// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package transfer

import (
	"context"
	"crypto/sha256"
	"errors"
	"fmt"
	"hash"
	"log"
	"strings"
	"sync"
	"time"

	"github.com/MemeLabs/strims/internal/api"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	transferv1 "github.com/MemeLabs/strims/pkg/apis/transfer/v1"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

const (
	peerSearchTickRate = time.Second
	peerSearchInterval = 30 * time.Second
)

type Control interface {
	Run()
	AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer
	RemovePeer(id uint64)
	Add(swarm *ppspp.Swarm, salt []byte) ID
	Find(swarm ppspp.SwarmID, salt []byte) (ID, *ppspp.Swarm, bool)
	Remove(id ID)
	List() []*transferv1.Transfer
	Publish(id ID, networkKey []byte)
	IsPublished(id ID, networkKey []byte) bool
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store dao.Store,
	observers *event.Observers,
) Control {
	return &control{
		ctx:    ctx,
		logger: logger,
		vpn:    vpn,
		store:  store,
		qosc:   vpn.VNIC().QOS().AddClass(1),

		events:    observers.Chan(),
		transfers: map[ID]*transfer{},
		peers:     map[uint64]*peer{},
		// candidates:  newCandidatePool(logger, vpn),
		searchQueue: newSearchQueue(int(peerSearchInterval / peerSearchTickRate)),
		networks:    hashmap.New[[]byte, *network](hashmap.NewByteInterface[[]byte]()),
		runner:      ppspp.NewRunner(ctx, logger),

		hackDialedPeers: map[kademlia.ID]struct{}{},
	}
}

// Control ...
type control struct {
	ctx    context.Context
	logger *zap.Logger
	vpn    *vpn.Host
	store  dao.Store
	qosc   *qos.Class

	lock      sync.Mutex
	events    chan any
	transfers map[ID]*transfer
	peers     map[uint64]*peer
	// candidates  *candidatePool
	searchQueue *searchQueue
	networks    hashmap.Map[[]byte, *network]
	runner      *ppspp.Runner

	hackDialedPeers map[kademlia.ID]struct{}
}

// Run ...
func (c *control) Run() {
	peerSerachTicker := timeutil.DefaultTickEmitter.Ticker(peerSearchTickRate)
	defer peerSerachTicker.Stop()

	// debugTicker := timeutil.DefaultTickEmitter.Ticker(30 * time.Second)
	// defer debugTicker.Stop()

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
		case t := <-peerSerachTicker.C:
			c.runPeerSearch(t)
		// case t := <-debugTicker.C:
		// 	c.debug(t)
		case <-c.ctx.Done():
			return
		}
	}
}

func (c *control) debug(t timeutil.Time) {
	var summary strings.Builder
	for id, p := range c.peers {
		snap := p.runnerPeer.MetricsSnapshot(t)
		fmt.Fprintf(&summary, "peer %d read: %d %d/s write: %d %d/s\n", id, snap.Read.Count, snap.Read.Rate, snap.Write.Count, snap.Write.Rate)
		for swarm, swarmSnap := range snap.Swarms {
			fmt.Fprintf(&summary, "swarm %s read: %d %d/s write: %d %d/s\n", swarm.ID(), swarmSnap.Read.Count, swarmSnap.Read.Rate, swarmSnap.Write.Count, swarmSnap.Write.Rate)
		}
	}
	log.Println(summary.String())
}

func (c *control) runPeerSearch(t timeutil.Time) {
	c.lock.Lock()
	defer c.lock.Unlock()
	for _, it := range c.searchQueue.Next() {
		go c.loadNetworkPeers(it.transfer, it.network)
	}
}

func (c *control) loadNetworkPeers(t *transfer, n *network) error {
	node, ok := c.vpn.Node(n.key)
	if !ok {
		return errors.New("network not found")
	}

	c.logger.Debug(
		"searching for peers",
		zap.Stringer("swarm", t.swarm.ID()),
		logutil.ByteHex("salt", t.salt),
	)

	ctx, cancel := context.WithTimeout(t.ctx, 10*time.Second)
	defer cancel()

	s, err := node.PeerIndex.Search(ctx, t.swarm.ID(), t.salt)
	if err != nil {
		c.logger.Debug(
			"searching for peers failed",
			zap.Error(err),
		)
		return err
	}

	var count int
	for p := range s {
		c.logger.Debug(
			"found peers",
			zap.Stringer("swarm", t.swarm.ID()),
			logutil.ByteHex("salt", t.salt),
			zap.Stringer("peer", p.HostID),
			zap.Bool("connected", c.vpn.VNIC().HasPeer(p.HostID)),
		)
		if c.vpn.VNIC().HasPeer(p.HostID) {
			continue
		}
		if _, ok := c.hackDialedPeers[p.HostID]; ok {
			continue
		}
		c.hackDialedPeers[p.HostID] = struct{}{}

		go node.PeerExchange.Connect(p.HostID)
		// c.candidates.addToCandidateThing(node, t, p)
		count++
	}

	c.logger.Debug(
		"finished searching for peers",
		zap.Stringer("swarm", t.swarm.ID()),
		logutil.ByteHex("salt", t.salt),
		zap.Int("count", count),
	)

	return nil
}

func (c *control) handleNetworkStart(networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.getOrInsertNetwork(networkKey)
}

func (c *control) handleNetworkStop(networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	n, ok := c.networks.Delete(networkKey)
	if !ok {
		return
	}

	c.searchQueue.DeleteNetwork(n)
}

func (c *control) handleNetworkPeerOpen(peerID uint64, networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	p, ok := c.peers[peerID]
	if !ok {
		return
	}

	n := c.getOrInsertNetwork(networkKey)
	n.peers[peerID] = p

	for _, t := range n.transfers {
		p.Announce(t)
	}
}

func (c *control) handleNetworkPeerClose(peerID uint64, networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	n, ok := c.networks.Get(networkKey)
	if !ok {
		return
	}

	delete(n.peers, peerID)

	// TODO: close peer transfers associated with this network?
}

// AddPeer ...
func (c *control) AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer {
	ctx, close := context.WithCancel(vnicPeer.Context())
	w := vnic.NewFrameWriter(vnicPeer.Link, vnic.TransferPort, c.qosc)
	cr, rp := c.runner.RunPeer(vnicPeer.HostID().Bytes(nil), w)
	p := &peer{
		logger:     c.logger.With(zap.Stringer("peer", vnicPeer.HostID())),
		ctx:        ctx,
		runnerPeer: rp,
		client:     client,
		transfers:  map[ID]*peerTransfer{},
	}
	vnicPeer.SetHandler(vnic.TransferPort, func(_ *vnic.Peer, f vnic.Frame) error {
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
func (c *control) RemovePeer(id uint64) {
	c.lock.Lock()
	p, ok := c.peers[id]
	delete(c.peers, id)
	c.lock.Unlock()

	if !ok {
		return
	}

	p.runnerPeer.Stop()
}

// Add ...
func (c *control) Add(swarm *ppspp.Swarm, salt []byte) ID {
	ctx, close := context.WithCancel(context.Background())

	t := &transfer{
		id:    NewID(swarm.ID(), salt),
		salt:  salt,
		ctx:   ctx,
		close: close,
		swarm: swarm,
	}

	c.lock.Lock()
	c.transfers[t.id] = t
	c.lock.Unlock()

	c.logger.Debug(
		"added swarm",
		logutil.ByteHex("id", t.id[:]),
		zap.Stringer("swarm", swarm.ID()),
	)

	return t.id
}

// Find ...
func (c *control) Find(swarmID ppspp.SwarmID, salt []byte) (ID, *ppspp.Swarm, bool) {
	id := NewID(swarmID, salt)

	c.lock.Lock()
	t, ok := c.transfers[id]
	c.lock.Unlock()

	if !ok {
		return ID{}, nil, false
	}
	return t.id, t.swarm, true
}

// Remove ...
func (c *control) Remove(id ID) {
	c.lock.Lock()
	defer c.lock.Unlock()

	t, ok := c.transfers[id]
	if !ok {
		return
	}

	delete(c.transfers, id)
	for it := c.networks.Iterate(); it.Next(); {
		delete(it.Value().transfers, id)
	}

	t.close()

	for _, p := range c.peers {
		p.Remove(t)
	}

	c.searchQueue.DeleteTransfer(t)

	c.logger.Debug(
		"closed swarm",
		logutil.ByteHex("id", t.id[:]),
		zap.Stringer("swarm", t.swarm.ID()),
	)
}

// List ...
func (c *control) List() []*transferv1.Transfer {
	c.lock.Lock()
	defer c.lock.Unlock()

	ts := make([]*transferv1.Transfer, len(c.transfers), 0)
	for _, t := range c.transfers {
		ts = append(ts, &transferv1.Transfer{
			Id: t.id[:],
		})
	}

	return ts
}

// Publish ...
func (c *control) Publish(id ID, networkKey []byte) {
	c.lock.Lock()
	defer c.lock.Unlock()

	t, ok := c.transfers[id]
	if !ok {
		return
	}

	n := c.getOrInsertNetwork(networkKey)

	if _, ok := n.transfers[id]; ok {
		return
	}
	n.transfers[id] = t

	for _, p := range n.peers {
		p.Announce(t)
	}

	c.searchQueue.Insert(t, n)

	go func() {
		node, ok := c.vpn.Node(networkKey)
		if !ok {
			return
		}

		err := node.PeerIndex.Publish(t.ctx, t.swarm.ID(), t.salt, 0)
		if err != nil {
			return
		}

		c.loadNetworkPeers(t, n)
	}()
}

func (c *control) IsPublished(id ID, networkKey []byte) bool {
	c.lock.Lock()
	defer c.lock.Unlock()
	n, ok := c.networks.Get(networkKey)
	return ok && n.transfers[id] != nil
}

func (c *control) getOrInsertNetwork(networkKey []byte) *network {
	n, ok := c.networks.Get(networkKey)
	if !ok {
		n = &network{
			key:       networkKey,
			peers:     map[uint64]*peer{},
			transfers: map[ID]*transfer{},
		}
		c.networks.Set(networkKey, n)
	}
	return n
}

// transfer ...
type transfer struct {
	id    ID
	salt  []byte
	ctx   context.Context
	close context.CancelFunc
	swarm *ppspp.Swarm
}

// network ...
type network struct {
	key       []byte
	peers     map[uint64]*peer
	transfers map[ID]*transfer
}

var idHashPool = &sync.Pool{
	New: func() any {
		return sha256.New()
	},
}

type ID [sha256.Size]byte

var NilID = ID{}

func (i ID) IsNil() bool {
	return i == NilID
}

func NewID(swarmID []byte, salt []byte) ID {
	h := idHashPool.Get().(hash.Hash)
	defer idHashPool.Put(h)

	h.Reset()
	h.Write(swarmID)
	h.Write(salt)

	var id ID
	h.Sum(id[:0])
	return id
}

func ParseID(b []byte) (id ID, err error) {
	if len(b) != len(id) {
		return id, errors.New("transfer id length mismatch")
	}
	copy(id[:], b)
	return
}
