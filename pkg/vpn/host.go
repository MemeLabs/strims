// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vpn

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/event"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"go.uber.org/zap"
)

const reservedPortCount = 1000
const recentMessageIDHistoryDefaultSize = 1024
const recentMessageIDHistoryTTL = 30 * time.Second
const maxMessageHops = 5
const maxMessageReplicas = 5
const qosClassWeight = 1

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
	return &Host{
		logger: logger,
		vnic:   vnic,
		qosc:   vnic.QOS().AddClass(qosClassWeight),
		nodes:  hashmap.New[[]byte, *Node](hashmap.NewByteInterface[[]byte]()),
		recentMessageIDs: newMessageIDLRU(
			recentMessageIDHistoryDefaultSize,
			recentMessageIDHistoryTTL,
		),
		hashTableStore: newHashTableStore(context.Background(), logger, vnic.ID()),
	}, nil
}

// Host ...
type Host struct {
	logger               *zap.Logger
	vnic                 *vnic.Host
	qosc                 *qos.Class
	nodesLock            sync.Mutex
	nodes                hashmap.Map[[]byte, *Node]
	networkObservers     event.Observer
	peerNetworkObservers event.Observer
	recentMessageIDs     *messageIDLRU
	hashTableStore       *HashTableStore
}

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
func (h *Host) AddNetwork(key []byte) (*Node, error) {
	h.nodesLock.Lock()
	defer h.nodesLock.Unlock()

	if _, ok := h.nodes.Get(key); ok {
		return nil, ErrDuplicateNetworkKey
	}

	logger := h.logger.With(logutil.ByteHex("network", key))

	qosc := h.qosc.AddClass(1)
	network := newNetwork(logger, h.vnic, qosc, key, h.recentMessageIDs)
	hashTable := newHashTable(logger, network, h.hashTableStore)
	peerIndex := newPeerIndex(logger, network)
	peerExchange := newPeerExchange(logger, network)

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
	h.nodes.Set(key, node)

	h.networkObservers.Emit(network)

	return node, nil
}

// RemoveNetwork ...
func (h *Host) RemoveNetwork(key []byte) error {
	h.nodesLock.Lock()
	defer h.nodesLock.Unlock()

	node, ok := h.nodes.Delete(key)
	if !ok {
		return ErrNetworkNotFound
	}

	node.PeerIndex.(*peerIndex).Close()
	node.Network.Close()
	return nil
}

// Node ...
func (h *Host) Node(key []byte) (*Node, bool) {
	h.nodesLock.Lock()
	defer h.nodesLock.Unlock()
	return h.nodes.Get(key)
}

// Nodes ...
func (h *Host) Nodes() []*Node {
	h.nodesLock.Lock()
	defer h.nodesLock.Unlock()
	return h.nodes.Values()
}

// Node ...
type Node struct {
	Host         *Host
	Network      *Network
	HashTable    HashTable
	PeerIndex    PeerIndex
	PeerExchange PeerExchange
}
