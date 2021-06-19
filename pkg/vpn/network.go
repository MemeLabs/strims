package vpn

import (
	"errors"
	"fmt"
	"math"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/randutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// newNetwork ...
func newNetwork(logger *zap.Logger, host *vnic.Host, qosc *qos.Class, certificate *certificate.Certificate, recentMessageIDs *messageIDLRU) *Network {
	return &Network{
		logger:           logger,
		host:             host,
		qosc:             qosc,
		certificate:      certificate,
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
	certificate      *certificate.Certificate
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
	return dao.GetRootCert(n.certificate).Key
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

	for i := 0; i < lastHopIndex; i++ {
		n.nextHop.Insert(m.Trailer.Entries[i].HostID, m.Trailer.Entries[lastHopIndex].HostID, lastHopIndex-i)
	}

	if ok := n.recentMessageIDs.Insert(m.ID()); !ok {
		return nil
	}

	return n.handleMessage(&m)
}

// Send ...
func (n *Network) Send(id kademlia.ID, port, srcPort uint16, b []byte) error {
	// n.logger.Debug(
	// 	"sending message",
	// 	zap.Stringer("dst", id),
	// 	zap.Uint16("srcPort", srcPort),
	// 	zap.Uint16("dstPort", port),
	// )
	return n.handleMessage(&Message{
		Header: MessageHeader{
			DstID:   id,
			DstPort: port,
			SrcPort: srcPort,
			Seq:     uint16(atomic.AddUint64(&n.seq, 1)),
			Length:  uint16(len(b)),
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

// SendProto ...
func (n *Network) SendProto(id kademlia.ID, port, srcPort uint16, msg proto.Message) error {
	b := pool.Get(proto.Size(msg))
	defer pool.Put(b)

	_, err := proto.MarshalOptions{}.MarshalAppend((*b)[:0], msg)
	if err != nil {
		return err
	}

	return n.Send(id, port, srcPort, *b)
}

func (n *Network) handleMessage(m *Message) error {
	// n.logger.Debug(
	// 	"received message",
	// 	zap.Stringer("src", m.SrcHostID()),
	// 	zap.Uint16("srcPort", m.Header.SrcPort),
	// 	zap.Uint16("dstPort", m.Header.DstPort),
	// )

	if m.Header.DstPort < reservedPortCount {
		if err := n.callHandler(m); err != nil {
			return err
		}
	} else if m.Header.DstID.Equals(n.host.ID()) {
		return n.callHandler(m)
	}

	if m.Trailer.Contains(n.host.ID()) {
		return nil
	}

	if m.Trailer.Hops >= maxMessageHops {
		return nil
	}

	return n.sendMessage(m)
}

func (n *Network) callHandler(m *Message) error {
	if h, ok := n.handlers[m.Header.DstPort]; ok {
		return h.HandleMessage(m)
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

	_, test := n.links.Get(m.Header.DstID)
	if ln != 0 && !conns[0].ID().Equals(m.Header.DstID) && test {
		panic("but why")
	}

	if ln != 0 && conns[0].ID().Equals(m.Header.DstID) {
		ln = 1
		// log.Println("using direct route")
	} else if id, ok := n.nextHop.Get(m.Header.DstID); ok {
		if conn, ok := n.links.Get(id); ok {
			if ln > 0 {
				copy(conns[1:], conns[:ln-1])
			}
			conns[0] = conn
			// ln = 1
			// log.Println("using known route", conn.ID().String())
		} else {
			// log.Println("no conn for known route")
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

		if m.Header.DstID.Equals(l.ID()) {
			break
		}

		if k++; k >= maxMessageReplicas-m.Trailer.Hops {
			break
		}
	}

	return nil
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
