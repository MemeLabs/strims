package vpn

import (
	"bytes"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/event"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	lru "github.com/hashicorp/golang-lru"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

const reservedPortCount = 1000
const recentMessageIDHistoryLength = 10000
const maxMessageHops = 5
const maxMessageReplicas = 5

// errors ...
var (
	ErrDuplicateNetworkKey      = errors.New("duplicate network key")
	ErrNetworkNotFound          = errors.New("network not found")
	ErrPeerClosed               = errors.New("peer closed")
	ErrNetworkBindingsEmpty     = errors.New("network bindings empty")
	ErrDiscriminatorBounds      = errors.New("discriminator out of range")
	ErrNetworkOwnerMismatch     = errors.New("init and network certificate key mismatch")
	ErrNetworkAuthorityMismatch = errors.New("network ca mismatch")
	ErrNetworkIDBounds          = errors.New("network id out of range")
)

// New ...
func New(logger *zap.Logger, vnicHost *vnic.Host, brokerFactory BrokerFactory) (*Host, error) {
	recentMessageIDs, err := lru.New(recentMessageIDHistoryLength)
	if err != nil {
		panic(err)
	}

	n := &Host{
		logger:           logger,
		VnicHost:         vnicHost,
		brokerFactory:    brokerFactory,
		recentMessageIDs: recentMessageIDs,
		hashTableStore:  vpn.NewHashTableStore(context.Background(), logger, vnicHost.ID()),
		peerIndexStore:  vpn.NewPeerIndexStore(context.Background(), logger, vnicHost.ID()),
	}

	vnicHost.AddPeerHandler(n.handlePeer)

	return n, nil
}

// Host ...
type Host struct {
	*vnic.Host
	logger               *zap.Logger
	brokerFactory        BrokerFactory
	clientsLock          sync.Mutex
	clients              clientMap
	networkObservers     event.Observable
	peerNetworkObservers event.Observable
	recentMessageIDs     *lru.Cache
	hashTableStore       *HashTableStore
	peerIndexStore       *PeerIndexStore
}

func (h *Host) handlePeer(p *vnic.Peer) {
	newBootstrap(h.logger, h, p)
}

// NotifyNetwork ...
func (h *Host) NotifyNetwork(ch chan *Network) {
	h.networkObservers.Notify(ch)
}

// StopNotifyingNetwork ...
func (h *Host) StopNotifyingNetwork(ch chan *Network) {
	h.networkObservers.StopNotifying(ch)
}

// NotifyPeerNetwork ...
func (h *Host) NotifyPeerNetwork(ch chan PeerNetwork) {
	h.peerNetworkObservers.Notify(ch)
}

// AddNetwork ...
func (h *Host) AddNetwork(cert *pb.Certificate) (*Client, error) {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()

	key := dao.GetRootCert(cert).Key
	if _, ok := h.clients.Get(key); ok {
		return nil, ErrDuplicateNetworkKey
	}

	network := newNetwork(h.logger, h.vnicHost, cert, h.recentMessageIDs)
	hashTable := NewHashTable(h.logger, network, h.hashTableStore)
	peerIndex := NewPeerIndex(h.logger, network, h.peerIndexStore)
	peerExchange := NewPeerExchange(h.logger, network)

	if err := network.SetHandler(vnic.HashTablePort, hashTable); err != nil {
		return nil, err
	}
	if err := network.SetHandler(vnic.PeerIndexPort, peerIndex); err != nil {
		return nil, err
	}
	if err := network.SetHandler(vnic.PeerExchangePort, peerExchange); err != nil {
		return nil, err
	}

	c := &Client{
		Host: h,
		Network:      network,
		HashTable:    hashTable,
		PeerIndex:    peerIndex,
		PeerExchange: peerExchange,
	}
	if !h.clients.Insert(c) {
		return nil, ErrDuplicateNetworkKey
	}

	h.networkObservers.Emit(network)

	return c, nil
}

// RemoveNetwork ...
func (h *Host) RemoveNetwork(n *Network) error {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()

	if _, ok := h.clients.Delete(n.Key()); !ok {
		return ErrNetworkNotFound
	}

	n.Close()
	return nil
}

// Networks ...
func (h *Host) Networks() []*Network {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()

	networks := make([]*Network, h.clients.Len(), 0)
	h.clients.Each(func(c *Client) bool {
		networks = append(networks, c.Network)
		return true
	})
	return networks
}

// NetworkKeys ...
func (h *Host) NetworkKeys() [][]byte {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()

	keys := make([][]byte, h.clients.Len(), 0)
	h.clients.Each(func(c *Client) bool {
		keys = append(keys, c.Network.Key())
		return true
	})
	return keys
}

// Client ...
func (h *Host) Client(key []byte) (*Client, bool) {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()
	return h.clients.Get(key); ok {
}

type clientMap struct {
	m llrb.LLRB
}

func (m *clientMap) Insert(c *Client) {
	m.m.InsertNoReplace(&clientMapItem{c.Network.Key(), c})
}

func (m *clientMap) Delete(k []byte) (*Client, bool) {
	if i := m.m.Delete(&clientMapItem{k: k}); i != nil {
		return i.(*clientMapItem).v, true
	}
	return nil, false
}

func (m *clientMap) Get(k []byte) (*Client, bool) {
	if i := m.m.Get(&clientMapItem{k: k}); i != nil {
		return i.(*clientMapItem).v, true
	}
	return nil, false
}

func (m *clientMap) Len() int {
	return m.m.Len()
}

func (m *clientMap) Each(f func(v *Client) bool) {
	m.m.AscendGreaterOrEqual(llrb.Inf(-1), func(i llrb.Item) bool {
		return f(i.(*clientMapItem).v)
	})
}

type clientMapItem struct {
	k []byte
	v *Client
}

func (t *clientMapItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(*clientMapItem); ok {
		return bytes.Compare(t.k, o.k) == -1
	}
	return !oi.Less(t)
}

// Client ...
type Client struct {
	Host *Host
	Network      *Network
	HashTable    HashTable
	PeerIndex    PeerIndex
	PeerExchange *PeerExchange
}
