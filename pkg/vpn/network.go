// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vpn

import (
	"errors"
	"fmt"
	"math"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/pool"
	"github.com/MemeLabs/strims/pkg/randutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// newNetwork ...
func newNetwork(logger *zap.Logger, host *vnic.Host, qosc *qos.Class, key []byte, recentMessageIDs *messageIDLRU) *Network {
	return &Network{
		logger:           logger,
		host:             host,
		qosc:             qosc,
		key:              key,
		recentMessageIDs: recentMessageIDs,
		links:            kademlia.NewKBucket(host.ID(), 20),
		handlers:         map[uint16]MessageHandler{},
		reservations:     map[uint16]struct{}{},
		nextHop:          newNextHopMap(),
	}
}

func newNextHopMap() *nextHopMap {
	return &nextHopMap{
		v: llrb.New(),
	}
}

type nextHopMap struct {
	mu sync.Mutex
	v  *llrb.LLRB
}

func (m *nextHopMap) Insert(target, nextHop kademlia.ID, distance int) {
	m.mu.Lock()
	defer m.mu.Unlock()

	prev := m.v.Get(&nextHopMapItem{target: target})
	if prev, ok := prev.(*nextHopMapItem); ok {
		if prev.distance <= distance {
			return
		}
	}
	m.v.ReplaceOrInsert(&nextHopMapItem{target, nextHop, distance})
}

func (m *nextHopMap) Get(target kademlia.ID) (kademlia.ID, bool) {
	m.mu.Lock()
	defer m.mu.Unlock()

	vi := m.v.Get(&nextHopMapItem{target: target})
	if v, ok := vi.(*nextHopMapItem); ok {
		return v.nextHop, true
	}
	return kademlia.ID{}, false
}

type nextHopMapItem struct {
	target   kademlia.ID
	nextHop  kademlia.ID
	distance int
}

func (h *nextHopMapItem) Less(o llrb.Item) bool {
	if o, ok := o.(*nextHopMapItem); ok {
		return h.target.Less(o.target)
	}
	return !o.Less(h)
}

// Network ...
type Network struct {
	logger           *zap.Logger
	host             *vnic.Host
	qosc             *qos.Class
	seq              uint64
	key              []byte
	recentMessageIDs *messageIDLRU
	linksLock        sync.Mutex
	links            *kademlia.KBucket
	handlersLock     sync.Mutex
	handlers         map[uint16]MessageHandler
	reservationsLock sync.Mutex
	reservations     map[uint16]struct{}
	connections      []*networkLink
	nextHop          *nextHopMap
}

// VNIC ...
func (n *Network) VNIC() *vnic.Host {
	return n.host
}

// SetHandler ...
func (n *Network) SetHandler(port uint16, h MessageHandler) error {
	n.handlersLock.Lock()
	n.reservationsLock.Lock()
	defer n.reservationsLock.Unlock()
	defer n.handlersLock.Unlock()

	n.reservations[port] = struct{}{}
	n.handlers[port] = h
	return nil
}

// RemoveHandler ...
func (n *Network) RemoveHandler(port uint16) {
	n.handlersLock.Lock()
	n.reservationsLock.Lock()
	defer n.reservationsLock.Unlock()
	defer n.handlersLock.Unlock()

	delete(n.reservations, port)
	delete(n.handlers, port)
}

// Handler ...
func (n *Network) Handler(port uint16) MessageHandler {
	n.handlersLock.Lock()
	defer n.handlersLock.Unlock()
	return n.handlers[port]
}

// ReservePort ...
func (n *Network) ReservePort() (uint16, error) {
	n.reservationsLock.Lock()
	defer n.reservationsLock.Unlock()

	for {
		port, err := randutil.Uint16n(math.MaxUint16 - reservedPortCount)
		if err != nil {
			return 0, err
		}
		port += reservedPortCount

		if _, ok := n.reservations[port]; !ok {
			n.reservations[port] = struct{}{}
			return port, nil
		}
	}
}

// ReleasePort ...
func (n *Network) ReleasePort(port uint16) {
	n.reservationsLock.Lock()
	defer n.reservationsLock.Unlock()

	delete(n.reservations, port)
}

// Close ...
func (n *Network) Close() {
	n.linksLock.Lock()
	defer n.linksLock.Unlock()

	for _, link := range n.links.Slice() {
		link.(*networkLink).peer.RemoveHandler(link.(*networkLink).port)
	}
	n.links.Reset()
}

// Key ...
func (n *Network) Key() []byte {
	return n.key
}

// AddPeer ...
func (n *Network) AddPeer(peer *vnic.Peer, srcPort, dstPort uint16) {
	n.linksLock.Lock()
	defer n.linksLock.Unlock()

	if _, ok := n.links.Get(peer.HostID()); ok {
		return
	}

	link := &networkLink{
		peer:        peer,
		port:        srcPort,
		FrameWriter: vnic.NewFrameWriter(peer.Link, dstPort, n.qosc),
	}
	peer.SetHandler(srcPort, n.handleFrame)

	n.links.Insert(link)
}

// RemovePeer ...
func (n *Network) RemovePeer(id kademlia.ID) {
	n.linksLock.Lock()
	defer n.linksLock.Unlock()

	link, ok := n.links.Get(id)
	if !ok {
		return
	}
	link.(*networkLink).peer.RemoveHandler(link.(*networkLink).port)

	n.links.Remove(id)
}

// HasPeer ...
func (n *Network) HasPeer(id kademlia.ID) bool {
	n.linksLock.Lock()
	defer n.linksLock.Unlock()

	_, ok := n.links.Get(id)
	return ok
}

// handleFrame ...
func (n *Network) handleFrame(p *vnic.Peer, f vnic.Frame) error {
	var m Message
	if _, err := m.Unmarshal(f.Body); err != nil {
		return fmt.Errorf("failed to read message from frame: %w", err)
	}

	lastHopIndex := m.Trailer.Hops - 1

	if lastHopIndex < 0 || !m.Trailer.Entries[lastHopIndex].HostID.Equals(p.HostID()) {
		return errors.New("message last hop trailer mismatch")
	}

	// TODO: verify messages here to prevent next hop/recent message index
	// poisoning. hop 0 always needs to be verified and hop n needs to be verified
	// if we are going to add it to the next hop index

	for i := 0; i < lastHopIndex; i++ {
		n.nextHop.Insert(m.Trailer.Entries[i].HostID, m.Trailer.Entries[lastHopIndex].HostID, lastHopIndex-i)
	}

	if ok := n.recentMessageIDs.Insert(m.ID()); !ok {
		return nil
	}

	return n.handleMessage(&m)
}

// Broadcast ...
func (n *Network) Broadcast(port, srcPort uint16, b []byte) error {
	return n.BroadcastWithFlags(port, srcPort, b, Mbroadcast)
}

// BroadcastWithFlags ...
func (n *Network) BroadcastWithFlags(port, srcPort uint16, b []byte, f uint16) error {
	return n.SendWithFlags(n.host.ID(), port, srcPort, b, f|Mbroadcast)
}

// Send ...
func (n *Network) Send(id kademlia.ID, port, srcPort uint16, b []byte) error {
	return n.SendWithFlags(id, port, srcPort, b, MstdFlags)
}

// SendWithFlags ...
func (n *Network) SendWithFlags(id kademlia.ID, port, srcPort uint16, b []byte, f uint16) error {
	var hs networkMessageHandler = (*Network).handleMessage
	if f&Mencrypt != 0 {
		hs = stackNetworkMessageHandler(hs, encryptMessage)
	}
	if f&Mcompress != 0 {
		hs = stackNetworkMessageHandler(hs, compressMessage)
	}

	// n.logger.Debug(
	// 	"sending message",
	// 	zap.Stringer("dst", id),
	// 	zap.Uint16("srcPort", srcPort),
	// 	zap.Uint16("dstPort", port),
	// )
	return hs(n, &Message{
		Header: MessageHeader{
			DstID:   id,
			DstPort: port,
			SrcPort: srcPort,
			Seq:     uint16(atomic.AddUint64(&n.seq, 1)),
			Length:  uint16(len(b)),
			Flags:   f,
		},
		Body: b,
		Trailer: MessageTrailer{
			Entries: []MessageTrailerEntry{
				{
					HostID: n.host.ID(),
				},
			},
		},
	})
}

// BroadcastProto ...
func (n *Network) BroadcastProto(port, srcPort uint16, msg proto.Message) error {
	return n.BroadcastProtoWithFlags(port, srcPort, msg, Mbroadcast)
}

// BroadcastProtoWithFlags ...
func (n *Network) BroadcastProtoWithFlags(port, srcPort uint16, msg proto.Message, f uint16) error {
	return n.SendProtoWithFlags(n.host.ID(), port, srcPort, msg, f|Mbroadcast)
}

// SendProto ...
func (n *Network) SendProto(id kademlia.ID, port, srcPort uint16, msg proto.Message) error {
	return n.SendProtoWithFlags(id, port, srcPort, msg, MstdFlags)
}

// SendProtoWithFlags ...
func (n *Network) SendProtoWithFlags(id kademlia.ID, port, srcPort uint16, msg proto.Message, f uint16) error {
	opt := proto.MarshalOptions{}
	b := pool.Get(opt.Size(msg))
	defer pool.Put(b)

	if _, err := opt.MarshalAppend((*b)[:0], msg); err != nil {
		return err
	}

	return n.SendWithFlags(id, port, srcPort, *b, f)
}

func (n *Network) handleMessage(m *Message) error {
	// n.logger.Debug(
	// 	"received message",
	// 	zap.Stringer("src", m.SrcHostID()),
	// 	zap.Uint16("srcPort", m.Header.SrcPort),
	// 	zap.Uint16("dstPort", m.Header.DstPort),
	// )

	if m.Header.Flags&Mbroadcast != 0 {
		if err := n.callHandler(m); err != nil {
			return err
		}
	} else if m.Header.DstID.Equals(n.host.ID()) {
		return n.callHandler(m)
	}

	if m.Header.Flags&Mnorelay != 0 && m.Trailer.Hops > 0 {
		return nil
	}
	if m.Trailer.Hops >= maxMessageHops {
		return nil
	}

	return n.sendMessage(m)
}

func (n *Network) callHandler(m *Message) error {
	if h := n.Handler(m.Header.DstPort); h != nil {
		var hs networkMessageHandler = func(n *Network, m *Message) error {
			return h.HandleMessage(m)
		}
		if m.Header.Flags&Mcompress != 0 {
			hs = stackNetworkMessageHandler(hs, uncompressMessage)
		}
		if m.Header.Flags&Mencrypt != 0 {
			hs = stackNetworkMessageHandler(hs, decryptMessage)
		}

		return hs(n, m.ShallowClone())
	}
	return nil
}

// sendMessage ...
func (n *Network) sendMessage(m *Message) error {
	b := pool.Get(m.Size())
	defer pool.Put(b)
	if _, err := m.Marshal(*b, n.host); err != nil {
		return err
	}

	var conns [maxMessageReplicas * 2]kademlia.Interface
	n.linksLock.Lock()
	ln := n.links.Closest(m.Header.DstID, conns[:])
	n.linksLock.Unlock()

	unicast := m.Header.Flags&Mbroadcast == 0
	if unicast {
		if ln != 0 && conns[0].ID().Equals(m.Header.DstID) {
			ln = 1
			// log.Println("using direct route")
		} else if id, ok := n.nextHop.Get(m.Header.DstID); ok {
			if conn, ok := n.links.Get(id); ok {
				k := ln
				for i := 0; i < ln; i++ {
					if conns[i] == conn {
						k = i
						break
					}
				}
				if k == ln && ln < len(conns) {
					ln++
				}
				copy(conns[1:], conns[:k])
				conns[0] = conn
				// ln = 1
				// log.Println("using known route", conn.ID().String())
			}
		}
	}

	var k int
	for _, li := range conns[:ln] {
		l := li.(*networkLink)

		if m.Trailer.Contains(l.ID()) {
			continue
		}

		// n.logger.Debug("writing frame", zap.Stringer("id", l.ID()))
		l.writeLock.Lock()
		if _, err := l.WriteFrame(*b); err != nil {
			n.logger.Debug("failed to write frame", zap.Error(err))
		}
		l.writeLock.Unlock()

		if unicast && m.Header.DstID.Equals(l.ID()) {
			break
		}

		if k++; k >= maxMessageReplicas-m.Trailer.Hops {
			break
		}
	}

	return nil
}

type messageFilter func(n *Network, m *Message, next networkMessageHandler) error

type networkMessageHandler func(n *Network, m *Message) error

func stackNetworkMessageHandler(hs networkMessageHandler, f messageFilter) networkMessageHandler {
	return func(n *Network, m *Message) error {
		return f(n, m, hs)
	}
}

// MessageHandler ...
type MessageHandler interface {
	HandleMessage(m *Message) error
}

// MessageHandlerFunc ...
func MessageHandlerFunc(fn func(*Message) error) MessageHandler {
	return &messageHandlerFunc{fn}
}

type messageHandlerFunc struct {
	handleMessage func(*Message) error
}

func (h *messageHandlerFunc) HandleMessage(msg *Message) error {
	return h.handleMessage(msg)
}

// TODO: handle eviction
type networkLink struct {
	peer      *vnic.Peer
	port      uint16
	writeLock sync.Mutex
	*vnic.FrameWriter
}

// ID ...
func (c *networkLink) ID() kademlia.ID {
	return c.peer.HostID()
}

// PeerNetwork ...
type PeerNetwork struct {
	Peer    *vnic.Peer
	Network *Network
}
