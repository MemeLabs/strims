package vpn

import (
	"fmt"
	"math"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/randutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	lru "github.com/hashicorp/golang-lru"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// newNetwork ...
func newNetwork(logger *zap.Logger, host *vnic.Host, certificate *pb.Certificate, recentMessageIDs *lru.Cache) *Network {
	return &Network{
		logger:           logger,
		host:             host,
		certificate:      certificate,
		recentMessageIDs: recentMessageIDs,
		links:            kademlia.NewKBucket(host.ID(), 20),
		handlers:         map[uint16]MessageHandler{},
		reservations:     map[uint16]struct{}{},
		done:             make(chan struct{}),
	}
}

// Network ...
type Network struct {
	logger           *zap.Logger
	host             *vnic.Host
	seq              uint64
	certificate      *pb.Certificate
	recentMessageIDs *lru.Cache
	linksLock        sync.Mutex
	links            *kademlia.KBucket
	closeOnce        sync.Once
	done             chan struct{}
	handlersLock     sync.Mutex
	handlers         map[uint16]MessageHandler
	reservationsLock sync.Mutex
	reservations     map[uint16]struct{}
	connections      []*networkLink
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
	n.closeOnce.Do(func() { close(n.done) })
}

// Done ...
func (n *Network) Done() <-chan struct{} {
	return n.done
}

// Key ...
func (n *Network) Key() []byte {
	return dao.GetRootCert(n.certificate).Key
}

// Certificate ...
func (n *Network) Certificate() *pb.Certificate {
	return n.certificate
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
		FrameWriter: vnic.NewFrameWriter(peer.Link, dstPort, peer.Link.MTU()),
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
	_, ok := n.links.Get(id)
	n.linksLock.Unlock()
	return ok
}

// handleFrame ...
func (n *Network) handleFrame(_ *vnic.Peer, f vnic.Frame) error {
	var m Message
	if _, err := m.Unmarshal(f.Body); err != nil {
		return fmt.Errorf("failed to read message from frame: %w", err)
	}

	if ok, _ := n.recentMessageIDs.ContainsOrAdd(m.ID(), struct{}{}); ok {
		return nil
	}

	return n.handleMessage(&m)
}

// Send ...
func (n *Network) Send(id kademlia.ID, port, srcPort uint16, b []byte) error {
	n.logger.Debug(
		"sending message",
		zap.Stringer("dst", id),
		zap.Uint16("srcPort", srcPort),
		zap.Uint16("dstPort", port),
	)
	return n.handleMessage(&Message{
		Header: MessageHeader{
			DstID:   id,
			DstPort: port,
			SrcPort: srcPort,
			Seq:     uint16(atomic.AddUint64(&n.seq, 1)),
			Length:  uint16(len(b)),
		},
		Body: b,
	})
}

// SendProto ...
func (n *Network) SendProto(id kademlia.ID, port, srcPort uint16, msg proto.Message) error {
	b := pool.Get(uint16(proto.Size(msg)))
	defer pool.Put(b)

	_, err := proto.MarshalOptions{}.MarshalAppend((*b)[:0], msg)
	if err != nil {
		return err
	}

	return n.Send(id, port, srcPort, *b)
}

func (n *Network) handleMessage(m *Message) error {
	if m.Header.DstID.Equals(n.host.ID()) {
		var src kademlia.ID
		if len(m.Trailers) != 0 {
			src = m.Trailers[0].HostID
		}
		n.logger.Debug(
			"received message",
			zap.Stringer("src", src),
			zap.Uint16("srcPort", m.Header.SrcPort),
			zap.Uint16("dstPort", m.Header.DstPort),
		)
		_, err := n.callHandler(m)
		return err
	}

	if m.Header.DstPort < reservedPortCount {
		forward, err := n.callHandler(m)
		if !forward || err != nil {
			return err
		}
	}

	if m.Trailers.Contains(n.host.ID()) {
		n.logger.Debug("dropping message we've already forwarded")
		return nil
	}

	if m.Hops() >= maxMessageHops {
		n.logger.Debug("dropping message after too many hops")
		return nil
	}

	return n.sendMessage(m)
}

func (n *Network) callHandler(m *Message) (bool, error) {
	if h, ok := n.handlers[m.Header.DstPort]; ok {
		return h.HandleMessage(m)
	}
	return true, nil
}

// sendMessage ...
func (n *Network) sendMessage(m *Message) error {
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)
	if _, err := m.Marshal(*b, n.host); err != nil {
		return err
	}

	var conns [maxMessageReplicas * 2]kademlia.Interface
	n.linksLock.Lock()
	ln := n.links.Closest(m.Header.DstID, conns[:])
	n.linksLock.Unlock()

	var k int
	for _, li := range conns[:ln] {
		l := li.(*networkLink)

		if m.Trailers.Contains(l.ID()) {
			continue
		}

		if _, err := l.WriteFrame(*b); err != nil {
			n.logger.Debug("failed to write frame", zap.Error(err))
		}

		if m.Header.DstID.Equals(l.ID()) {
			break
		}

		if k++; k >= maxMessageHops {
			break
		}
	}

	return nil
}

// MessageHandler ...
type MessageHandler interface {
	HandleMessage(m *Message) (bool, error)
}

// TODO: handle eviction
type networkLink struct {
	peer *vnic.Peer
	port uint16
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
