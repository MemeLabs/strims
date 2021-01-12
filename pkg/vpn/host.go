package vpn

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/event"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	lru "github.com/hashicorp/golang-lru"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

const reservedPortCount = 1000
const recentMessageIDHistoryLength = 100000
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
func New(logger *zap.Logger, vnic *vnic.Host) (*Host, error) {
	recentMessageIDs, err := lru.New(recentMessageIDHistoryLength)
	if err != nil {
		return nil, err
	}

	return &Host{
		logger:           logger,
		vnic:             vnic,
		recentMessageIDs: recentMessageIDs,
		hashTableStore:   newHashTableStore(context.Background(), logger, vnic.ID()),
		peerIndexStore:   newPeerIndexStore(context.Background(), logger, vnic.ID()),
	}, nil
}

// Host ...
type Host struct {
	logger               *zap.Logger
	vnic                 *vnic.Host
	clientsLock          sync.Mutex
	nodes                nodeMap
	networkObservers     event.Observer
	peerNetworkObservers event.Observer
	recentMessageIDs     *lru.Cache
	hashTableStore       *HashTableStore
	peerIndexStore       *PeerIndexStore
}

// TODO: get networks
// TODO: get peers
// TODO: get peers by network

// VNIC ...
func (h *Host) VNIC() *vnic.Host {
	return h.vnic
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
func (h *Host) AddNetwork(cert *certificate.Certificate) (*Node, error) {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()

	key := dao.GetRootCert(cert).Key
	if _, ok := h.nodes.Get(key); ok {
		return nil, ErrDuplicateNetworkKey
	}

	network := newNetwork(h.logger, h.vnic, cert, h.recentMessageIDs)
	hashTable := newHashTable(h.logger, network, h.hashTableStore)
	peerIndex := newPeerIndex(h.logger, network, h.peerIndexStore)
	peerExchange := newPeerExchange(h.logger, network)

	if err := network.SetHandler(vnic.HashTablePort, hashTable); err != nil {
		return nil, err
	}
	if err := network.SetHandler(vnic.PeerIndexPort, peerIndex); err != nil {
		return nil, err
	}
	if err := network.SetHandler(vnic.PeerExchangePort, peerExchange); err != nil {
		return nil, err
	}

	node := &Node{
		Host:         h,
		Network:      network,
		HashTable:    hashTable,
		PeerIndex:    peerIndex,
		PeerExchange: peerExchange,
	}
	h.nodes.Insert(node)

	h.networkObservers.Emit(network)

	return node, nil
}

// RemoveNetwork ...
func (h *Host) RemoveNetwork(key []byte) error {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()

	client, ok := h.nodes.Delete(key)
	if !ok {
		return ErrNetworkNotFound
	}

	client.Network.Close()
	return nil
}

// Node ...
func (h *Host) Node(key []byte) (*Node, bool) {
	h.clientsLock.Lock()
	defer h.clientsLock.Unlock()
	return h.nodes.Get(key)
}

type nodeMap struct {
	m llrb.LLRB
}

func (m *nodeMap) Insert(c *Node) {
	m.m.InsertNoReplace(&nodeMapItem{c.Network.Key(), c})
}

func (m *nodeMap) Delete(k []byte) (*Node, bool) {
	if i := m.m.Delete(&nodeMapItem{k: k}); i != nil {
		return i.(*nodeMapItem).v, true
	}
	return nil, false
}

func (m *nodeMap) Get(k []byte) (*Node, bool) {
	if i := m.m.Get(&nodeMapItem{k: k}); i != nil {
		return i.(*nodeMapItem).v, true
	}
	return nil, false
}

func (m *nodeMap) Len() int {
	return m.m.Len()
}

func (m *nodeMap) Each(f func(v *Node) bool) {
	m.m.AscendGreaterOrEqual(llrb.Inf(-1), func(i llrb.Item) bool {
		return f(i.(*nodeMapItem).v)
	})
}

type nodeMapItem struct {
	k []byte
	v *Node
}

func (t *nodeMapItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(*nodeMapItem); ok {
		return bytes.Compare(t.k, o.k) == -1
	}
	return !oi.Less(t)
}

// Node ...
type Node struct {
	Host         *Host
	Network      *Network
	HashTable    HashTable
	PeerIndex    PeerIndex
	PeerExchange PeerExchange
}
