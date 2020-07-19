package service

import (
	"bytes"
	"context"
	"fmt"
	"reflect"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

var nextServiceOptionID uint32

// NetworkServices ...
type NetworkServices struct {
	Host         *vpn.Host
	Network      *vpn.Network
	HashTable    vpn.HashTable
	PeerIndex    vpn.PeerIndex
	PeerExchange *vpn.PeerExchange
	Swarms       SwarmNetwork
	Directory    Directory
}

// NewNetworkController ...
func NewNetworkController(logger *zap.Logger, host *vpn.Host, store *dao.ProfileStore) (*NetworkController, error) {
	networks := vpn.NewNetworks(logger, host)

	c := &NetworkController{
		logger:          logger,
		store:           store,
		host:            host,
		networks:        networks,
		hashTableStore:  vpn.NewHashTableStore(context.Background(), logger, host.ID()),
		peerIndexStore:  vpn.NewPeerIndexStore(context.Background(), logger, host.ID()),
		swarmController: newSwarmController(logger, host, networks),
		eventEmitter:    newNetworkEventEmitter(),
	}

	if err := c.startProfileNetworks(); err != nil {
		return nil, err
	}

	return c, nil
}

// NetworkController ...
type NetworkController struct {
	logger              *zap.Logger
	store               *dao.ProfileStore
	networkServicesLock sync.Mutex
	networkServices     networkServicesMap
	host                *vpn.Host
	networks            *vpn.Networks
	hashTableStore      *vpn.HashTableStore
	peerIndexStore      *vpn.PeerIndexStore
	swarmController     *swarmController
	eventEmitter        *networkEventEmitter
}

type pbNetworks []*pb.Network

func (n pbNetworks) Find(key []byte) *pb.Network {
	for _, in := range n {
		if bytes.Equal(in.Key.Public, key) {
			return in
		}
	}
	return nil
}

func (c *NetworkController) startProfileNetworks() error {
	memberships, err := dao.GetNetworkMemberships(c.store)
	if err != nil {
		return err
	}
	networks, err := dao.GetNetworks(c.store)
	if err != nil {
		return err
	}

	for _, m := range memberships {
		network := pbNetworks(networks).Find(dao.GetRootCert(m.Certificate).Key)
		if network == nil {
			_, err = c.StartNetwork(m.Certificate, WithMemberServices(c.logger))
		} else {
			_, err = c.StartNetwork(m.Certificate, WithOwnerServices(c.logger, c.store, network))
		}
		if err != nil {
			c.logger.Error("failed to start network", zap.Error(err))
			continue
		}
	}

	return nil
}

// NetworkServices ...
func (c *NetworkController) NetworkServices(key []byte) (*NetworkServices, bool) {
	c.networkServicesLock.Lock()
	defer c.networkServicesLock.Unlock()
	return c.networkServices.Get(key)
}

// StartNetwork ...
func (c *NetworkController) StartNetwork(cert *pb.Certificate, opts ...NetworkOption) (*NetworkServices, error) {
	network, err := c.networks.AddNetwork(cert)
	if err != nil {
		return nil, err
	}

	hashTable := vpn.NewHashTable(c.logger, network, c.hashTableStore)
	peerIndex := vpn.NewPeerIndex(c.logger, network, c.peerIndexStore)
	peerExchange := vpn.NewPeerExchange(c.logger, network)

	if err := network.SetHandler(vpn.HashTablePort, hashTable); err != nil {
		return nil, err
	}
	if err := network.SetHandler(vpn.PeerIndexPort, peerIndex); err != nil {
		return nil, err
	}
	if err := network.SetHandler(vpn.PeerExchangePort, peerExchange); err != nil {
		return nil, err
	}

	svc := &NetworkServices{
		Host:         c.host,
		Network:      network,
		HashTable:    hashTable,
		PeerIndex:    peerIndex,
		PeerExchange: peerExchange,
		Swarms:       c.swarmController.AddNetwork(network),
	}

	for _, opt := range opts {
		if err := opt(svc); err != nil {
			return nil, err
		}
	}

	c.networkServicesLock.Lock()
	c.networkServices.Insert(dao.GetRootCert(cert).Key, svc)
	c.networkServicesLock.Unlock()

	c.eventEmitter.EmitOpen(svc)

	return svc, nil
}

// NetworkOption ...
type NetworkOption func(svc *NetworkServices) error

// WithOwnerServices ...
func WithOwnerServices(logger *zap.Logger, store *dao.ProfileStore, network *pb.Network) NetworkOption {
	return func(svc *NetworkServices) error {
		lock := dao.NewMutex(store, []byte(fmt.Sprintf("network:%d", network.Id)))

		directory, err := NewDirectoryServer(logger, lock, svc, network.Key)
		if err != nil {
			return err
		}
		svc.Directory = directory

		return nil
	}
}

// WithMemberServices ...
func WithMemberServices(logger *zap.Logger) NetworkOption {
	return func(svc *NetworkServices) error {
		directory, err := NewDirectoryClient(logger, svc, svc.Network.CAKey())
		if err != nil {
			return err
		}
		svc.Directory = directory

		return nil
	}
}

// StopNetwork ...
func (c *NetworkController) StopNetwork(cert *pb.Certificate) error {
	key := dao.GetRootCert(cert).Key

	c.networkServicesLock.Lock()
	defer c.networkServicesLock.Unlock()
	svc, ok := c.networkServices.Get(key)
	if !ok {
		return nil
	}
	c.networkServices.Delete(key)

	return c.networks.RemoveNetwork(svc.Network)
}

// Events ...
func (c *NetworkController) Events() <-chan *pb.NetworkEvent {
	return c.eventEmitter.events
}

func newNetworkEventEmitter() *networkEventEmitter {
	e := &networkEventEmitter{
		services: make(chan *NetworkServices),
		events:   make(chan *pb.NetworkEvent, 1024),
		nextID:   1,
	}

	go e.pump()

	return e
}

type networkEventEmitter struct {
	services chan *NetworkServices
	events   chan *pb.NetworkEvent
	cases    []reflect.SelectCase
	ids      []uint64
	nextID   uint64
}

func (e *networkEventEmitter) pump() {
	e.cases = append(e.cases, reflect.SelectCase{
		Dir:  reflect.SelectRecv,
		Chan: reflect.ValueOf(e.services),
	})
	e.ids = append(e.ids, 0)

	for {
		i, v, ok := reflect.Select(e.cases)

		if i == 0 {
			e.handleOpen(v.Interface().(*NetworkServices))
			continue
		}

		if !ok {
			e.handleClose(i)
			continue
		}

		e.handleDirectoryEvent(i, v.Interface().(*pb.DirectoryServerEvent))
	}
}

func (e *networkEventEmitter) handleOpen(svc *NetworkServices) {
	e.events <- &pb.NetworkEvent{
		Body: &pb.NetworkEvent_NetworkOpen_{
			NetworkOpen: &pb.NetworkEvent_NetworkOpen{
				NetworkId:  e.nextID,
				NetworkKey: svc.Network.CAKey(),
			},
		},
	}

	e.cases = append(e.cases, reflect.SelectCase{
		Dir:  reflect.SelectRecv,
		Chan: reflect.ValueOf(svc.Directory.Events()),
	})

	e.ids = append(e.ids, e.nextID)
	e.nextID++
}

func (e *networkEventEmitter) handleClose(i int) {
	e.events <- &pb.NetworkEvent{
		Body: &pb.NetworkEvent_NetworkClose_{
			NetworkClose: &pb.NetworkEvent_NetworkClose{
				NetworkId: e.ids[i],
			},
		},
	}

	copy(e.cases[i:], e.cases[i+1:])
	e.cases = e.cases[:len(e.cases)-1]

	copy(e.ids[i:], e.ids[i+1:])
	e.ids = e.ids[:len(e.ids)-1]
}

func (e *networkEventEmitter) handleDirectoryEvent(i int, event *pb.DirectoryServerEvent) {
	e.events <- &pb.NetworkEvent{
		Body: &pb.NetworkEvent_DirectoryEvent_{
			DirectoryEvent: &pb.NetworkEvent_DirectoryEvent{
				NetworkId: e.ids[i],
				Event:     event,
			},
		},
	}
}

func (e *networkEventEmitter) EmitOpen(svc *NetworkServices) {
	e.services <- svc
}

type networkServicesMap struct {
	m llrb.LLRB
}

func (m *networkServicesMap) Insert(k []byte, v *NetworkServices) {
	m.m.InsertNoReplace(networkServicesMapItem{k, v})
}

func (m *networkServicesMap) Delete(k []byte) {
	m.m.Delete(networkServicesMapItem{k, nil})
}

func (m *networkServicesMap) Get(k []byte) (*NetworkServices, bool) {
	if it := m.m.Get(networkServicesMapItem{k, nil}); it != nil {
		return it.(networkServicesMapItem).v, true
	}
	return nil, false
}

type networkServicesMapItem struct {
	k []byte
	v *NetworkServices
}

func (t networkServicesMapItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(networkServicesMapItem); ok {
		return bytes.Compare(t.k, o.k) == -1
	}
	return !oi.Less(t)
}
