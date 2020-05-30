package vpn

import (
	"bytes"
	"errors"
	"math"
	"sort"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/event"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	lru "github.com/hashicorp/golang-lru"
	"go.uber.org/zap"
)

const recentMessageIDHistoryLength = 10000
const maxMessageHops = 5
const maxMessageReplicas = 5

// NewNetworks ...
func NewNetworks(host *Host) *Networks {
	recentMessageIDs, err := lru.New(recentMessageIDHistoryLength)
	if err != nil {
		panic(err)
	}

	n := &Networks{
		host:             host,
		recentMessageIDs: recentMessageIDs,
	}

	go n.handlePeers()

	return n
}

// Networks ...
type Networks struct {
	host                 *Host
	networksLock         sync.Mutex
	networks             []*Network
	networkObservers     event.Observable
	peerNetworkObservers event.Observable
	recentMessageIDs     *lru.Cache
}

func (h *Networks) handlePeers() {
	ch := make(chan *Peer, 16)
	h.host.NotifyPeer(ch)
	for p := range ch {
		newNetworkBootstrap(h, p)
	}
}

// NotifyNetwork ...
func (h *Networks) NotifyNetwork(ch chan *Network) {
	h.networkObservers.Notify(ch)
}

// StopNotifyingNetwork ...
func (h *Networks) StopNotifyingNetwork(ch chan *Network) {
	h.networkObservers.StopNotifying(ch)
}

// NotifyPeerNetwork ...
func (h *Networks) NotifyPeerNetwork(ch chan PeerNetwork) {
	h.peerNetworkObservers.Notify(ch)
}

// AddNetwork ...
func (h *Networks) AddNetwork(cert *pb.Certificate) (*Network, error) {
	n := NewNetwork(h.host, cert, h.recentMessageIDs)

	h.networksLock.Lock()
	defer h.networksLock.Unlock()

	end := len(h.networks)
	i, ok := h.findIndexByKey(n.CAKey())
	if ok {
		return nil, errors.New("duplicate network key")
	}

	h.networks = append(h.networks, n)
	if i != end {
		copy(h.networks[i+1:], h.networks[i:])
		h.networks[i] = n
	}

	h.networkObservers.Emit(n)

	return n, nil
}

// RemoveNetwork ...
func (h *Networks) RemoveNetwork(n *Network) error {
	h.networksLock.Lock()
	defer h.networksLock.Unlock()

	i, ok := h.findIndexByKey(n.CAKey())
	if !ok || h.networks[i] != n {
		return errors.New("network not found")
	}

	copy(h.networks[i:], h.networks[i+1:])
	h.networks = h.networks[:len(h.networks)-1]

	n.Close()
	return nil
}

// Networks ...
func (h *Networks) Networks() []*Network {
	h.networksLock.Lock()
	defer h.networksLock.Unlock()
	c := make([]*Network, len(h.networks))
	copy(c, h.networks)
	return c
}

// NetworkKeys ...
func (h *Networks) NetworkKeys() [][]byte {
	h.networksLock.Lock()
	defer h.networksLock.Unlock()

	keys := make([][]byte, len(h.networks))
	for i, n := range h.networks {
		keys[i] = n.CAKey()
	}
	return keys
}

// FindByKey ...
func (h *Networks) FindByKey(key []byte) (*Network, bool) {
	h.networksLock.Lock()
	defer h.networksLock.Unlock()

	i, ok := h.findIndexByKey(key)
	if !ok {
		return nil, false
	}
	return h.networks[i], true
}

func (h *Networks) findIndexByKey(key []byte) (int, bool) {
	end := len(h.networks)
	i := sort.Search(end, func(i int) bool {
		return bytes.Compare(key, h.networks[i].CAKey()) >= 0
	})
	return i, i < end && bytes.Compare(key, h.networks[i].CAKey()) == 0
}

func newNetworkBootstrap(n *Networks, peer *Peer) *networkBootstrap {
	b := &networkBootstrap{
		networks:   n,
		peer:       peer,
		links:      make(map[*Network]*networkLink),
		handshakes: make(chan *pb.NetworkHandshake),
	}

	go b.doBootstrap()

	return b
}

type networkBootstrap struct {
	networks   *Networks
	peer       *Peer
	links      map[*Network]*networkLink
	handshakes chan *pb.NetworkHandshake
}

func (h *networkBootstrap) doBootstrap() (err error) {
	defer h.removeLinks()

	ch := NewFrameReadWriter(h.peer.Link, 0, h.peer.Link.MTU())
	h.peer.SetHandler(0, ch.HandleFrame)
	defer h.peer.RemoveHandler(0)
	defer ch.Close()

	networks := make(chan *Network, 1)
	h.networks.NotifyNetwork(networks)
	defer h.networks.StopNotifyingNetwork(networks)

	bch := NewFrameReadWriter(h.peer.Link, 1, h.peer.Link.MTU())
	h.peer.SetHandler(1, bch.HandleFrame)
	defer h.peer.RemoveHandler(1)
	defer bch.Close()

	broker, err := h.networks.host.networkBroker.BrokerPeer(bch)
	if err != nil {
		return err
	}
	defer broker.Close()

	if err := h.initBroker(broker); err != nil {
		return err
	}

	go h.readHandshakes(ch)

	for {
		select {
		case keys := <-broker.Keys():
			err = h.negotiateNetworksAsSender(ch, keys)
		case handshake := <-h.handshakes:
			err = h.negotiateNetworksAsReceiver(ch, handshake)
		case <-networks:
			err = h.initBroker(broker)
		case <-broker.InitRequired():
			err = h.initBroker(broker)
		case <-h.peer.Done():
			err = errors.New("peer closed")
		}
		if err != nil {
			return err
		}
	}
}

func (h *networkBootstrap) removeLinks() {
	for n, l := range h.links {
		h.networks.host.logger.Debug(
			"removing peer from network",
			logutil.ByteHex("peer", l.hostID.Bytes(nil)),
			logutil.ByteHex("network", certificateParentKey(n.certificate)),
		)

		n.RemoveLink(l)
	}
}

func (h *networkBootstrap) readHandshakes(ch *FrameReadWriter) error {
	for {
		var handshake pb.NetworkHandshake
		if err := ReadProtoStream(ch, &handshake); err != nil {
			h.peer.Close()
			return err
		}
		h.handshakes <- &handshake
	}
}

func (h *networkBootstrap) initBroker(b NetworkBrokerPeer) error {
	return b.Init(h.networks.host.discriminator, h.networks.NetworkKeys())
}

func (h *networkBootstrap) negotiateNetworksAsSender(ch *FrameReadWriter, keys [][]byte) error {
	networkBindings, err := h.sendNetworkBindings(ch, keys)
	if err != nil {
		return err
	}
	handshake := <-h.handshakes
	peerNetworkBindings := handshake.GetNetworkBindings()
	if _, err = h.verifyNetworkBindings(peerNetworkBindings); err != nil {
		return err
	}
	return h.handleNetworkBindings(peerNetworkBindings.Discriminator, networkBindings, peerNetworkBindings.NetworkBindings)
}

func (h *networkBootstrap) negotiateNetworksAsReceiver(ch *FrameReadWriter, handshake *pb.NetworkHandshake) error {
	peerNetworkBindings := handshake.GetNetworkBindings()
	keys, err := h.verifyNetworkBindings(peerNetworkBindings)
	if err != nil {
		return err
	}
	networkBindings, err := h.sendNetworkBindings(ch, keys)
	if err != nil {
		return err
	}
	return h.handleNetworkBindings(peerNetworkBindings.Discriminator, networkBindings, peerNetworkBindings.NetworkBindings)
}

func (h *networkBootstrap) sendNetworkBindings(ch *FrameReadWriter, keys [][]byte) ([]*pb.NetworkHandshake_NetworkBinding, error) {
	var bindings []*pb.NetworkHandshake_NetworkBinding

	for _, key := range keys {
		n, ok := h.networks.FindByKey(key)
		if !ok {
			return nil, errors.New("network key not found")
		}
		if _, ok := h.links[n]; ok {
			continue
		}

		port, err := h.peer.ReservePort()
		if err != nil {
			return nil, err
		}

		bindings = append(
			bindings,
			&pb.NetworkHandshake_NetworkBinding{
				Port:        uint32(port),
				Certificate: n.certificate,
			},
		)
	}
	err := WriteProtoStream(ch, &pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_NetworkBindings_{
			NetworkBindings: &pb.NetworkHandshake_NetworkBindings{
				Discriminator:   uint32(h.networks.host.discriminator),
				NetworkBindings: bindings,
			},
		},
	})
	if err != nil {
		return nil, err
	}
	if err := ch.Flush(); err != nil {
		return nil, err
	}
	return bindings, nil
}

func (h *networkBootstrap) verifyNetworkBindings(bindings *pb.NetworkHandshake_NetworkBindings) ([][]byte, error) {
	if bindings == nil {
		return nil, errors.New("network bindings empty")
	}

	keys := make([][]byte, len(bindings.NetworkBindings))
	for i, b := range bindings.NetworkBindings {
		if err := dao.VerifyCertificate(b.Certificate); err != nil {
			return nil, err
		}
		keys[i] = certificateParentKey(b.Certificate)
	}
	return keys, nil
}

// NewNetwork ...
func NewNetwork(host *Host, certificate *pb.Certificate, recentMessageIDs *lru.Cache) *Network {
	return &Network{
		host:             host,
		certificate:      certificate,
		recentMessageIDs: recentMessageIDs,
		links:            kademlia.NewKBucket(host.ID(), 20),
		handlers:         map[uint16]MessageHandler{},
		reservations:     map[uint16]struct{}{},
		done:             make(chan struct{}),
	}
}

func (h *networkBootstrap) handleNetworkBindings(discriminator uint32, networkBindings, peerNetworkBindings []*pb.NetworkHandshake_NetworkBinding) error {

	if discriminator > uint32(math.MaxUint16) {
		return errors.New("discriminator out of range")
	}

	hostID, err := NewHostID(h.peer.Certificate.Key, uint16(discriminator))
	if err != nil {
		return err
	}

	for i, pb := range peerNetworkBindings {
		b := networkBindings[i]

		if bytes.Compare(h.peer.Certificate.Key, pb.Certificate.Key) != 0 {
			return errors.New("init and network certificate key mismatch")
		}
		if bytes.Compare(certificateParentKey(b.Certificate), certificateParentKey(pb.Certificate)) != 0 {
			return errors.New("network ca mismatch")
		}
		if pb.Port > uint32(math.MaxUint16) {
			return errors.New("network id out of range")
		}

		n, ok := h.networks.FindByKey(certificateParentKey(pb.Certificate))
		if !ok {
			return errors.New("network key not found")
		}

		h.networks.host.logger.Debug(
			"adding peer to network",
			logutil.ByteHex("peer", hostID.Bytes(nil)),
			logutil.ByteHex("network", certificateParentKey(pb.Certificate)),
			zap.Uint32("localPort", b.Port),
			zap.Uint32("remotePort", pb.Port),
		)

		link := &networkLink{
			hostID:          hostID,
			FrameReadWriter: NewFrameReadWriter(h.peer.Link, uint16(pb.Port), h.peer.Link.MTU()),
		}
		h.links[n] = link
		n.AddLink(link)

		h.peer.SetHandler(uint16(b.Port), func(p *Peer, f Frame) error {
			n.HandleFrame(f)
			return nil
		})

		h.networks.peerNetworkObservers.Emit(PeerNetwork{h.peer, n})
	}
	return nil
}

// Network ...
type Network struct {
	host             *Host
	seq              uint64
	certificate      *pb.Certificate
	recentMessageIDs *lru.Cache
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
		port, err := randUint16(math.MaxUint16 - reservedPortCount)
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

// CAKey ...
func (n *Network) CAKey() []byte {
	return certificateParentKey(n.certificate)
}

// AddLink ...
func (n *Network) AddLink(link *networkLink) {
	n.links.Insert(link)
}

// RemoveLink ...
func (n *Network) RemoveLink(link *networkLink) {
	n.links.Remove(link.ID())
}

// HandleFrame ...
func (n *Network) HandleFrame(f Frame) {
	var m Message
	if _, err := m.Unmarshal(f.Body); err != nil {
		n.host.logger.Debug("failed to read frame", zap.Error(err))
		return
	}

	if ok, _ := n.recentMessageIDs.ContainsOrAdd(m.ID(), struct{}{}); ok {
		return
	}

	n.handleMessage(&m)
}

// Send ...
func (n *Network) Send(id kademlia.ID, port, srcPort uint16, b []byte) error {
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

func (n *Network) handleMessage(m *Message) error {
	if m.Header.DstID.Equals(n.host.ID()) {
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
		n.host.logger.Debug("dropping message we've already forwarded")
		return nil
	}

	if m.Hops() >= maxMessageHops {
		n.host.logger.Debug("dropping message eafter too many hops")
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
	b := frameBuffer(uint16(m.Size()))
	defer freeFrameBuffer(b)
	if _, err := m.Marshal(b, n.host); err != nil {
		return err
	}

	var conns [maxMessageReplicas * 2]kademlia.Interface
	ln := n.links.Closest(m.Header.DstID, conns[:])
	var k int
	for _, li := range conns[:ln] {
		l := li.(*networkLink)

		if m.Trailers.Contains(l.ID()) {
			// log.Println("skipping host found in trailers")
			continue
		}

		if _, err := l.WriteFrame(b); err != nil {
			n.host.logger.Debug("failed to write frame", zap.Error(err))
		}

		if m.Header.DstID.Equals(l.ID()) {
			// log.Println("found the to peer, done forwarding", l)
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
	hostID kademlia.ID
	*FrameReadWriter
}

// ID ...
func (c *networkLink) ID() kademlia.ID {
	return c.hostID
}

// PeerNetwork ...
type PeerNetwork struct {
	Peer    *Peer
	Network *Network
}

func certificateParentKey(c *pb.Certificate) []byte {
	if c.GetParent() == nil {
		return nil
	}
	return c.GetParent().GetKey()
}
