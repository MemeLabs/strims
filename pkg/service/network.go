package service

import (
	"bytes"
	"context"
	"errors"
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
}

// NewNetworksController ...
func NewNetworksController(logger *zap.Logger, store *dao.ProfileStore) *NetworksController {
	return &NetworksController{
		logger: logger,
		store:  store,
	}
}

// NetworksController ...
type NetworksController struct {
	logger              *zap.Logger
	store               *dao.ProfileStore
	networkServicesLock sync.Mutex
	networkServices     networkServicesMap
	host                *vpn.Host
	networks            *vpn.Networks
	hashTableStore      *vpn.HashTableStore
	peerIndexStore      *vpn.PeerIndexStore
	swarmController     *swarmController
}

// WithNetworkController ...
func WithNetworkController(c *NetworksController) vpn.HostOption {
	return func(h *vpn.Host) error {
		c.host = h
		c.networks = vpn.NewNetworks(h)
		c.hashTableStore = vpn.NewHashTableStore(context.Background(), c.logger, h.ID())
		c.peerIndexStore = vpn.NewPeerIndexStore(context.Background(), c.logger, h.ID())
		c.swarmController = newSwarmController(c.logger, h, c.networks)

		if err := c.startProfileNetworks(); err != nil {
			panic(err)
		}

		return nil
	}
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

func (c *NetworksController) startProfileNetworks() error {
	memberships, err := c.store.GetNetworkMemberships()
	if err != nil {
		return err
	}
	networks, err := c.store.GetNetworks()
	if err != nil {
		return err
	}

	for _, m := range memberships {
		_, err := c.StartNetwork(m.Certificate)
		if err != nil {
			c.logger.Error("failed to start network", zap.Error(err))
			continue
		}

		options := pbNetworks(networks).Find(dao.GetRootCert(m.Certificate).Key)
		if options != nil {
			c.StartNetworkAuthorityServices(m.Certificate, options)
		}
	}

	return nil
}

// NetworkServices ...
func (c *NetworksController) NetworkServices(key []byte) (*NetworkServices, bool) {
	c.networkServicesLock.Lock()
	defer c.networkServicesLock.Unlock()
	return c.networkServices.Get(key)
}

// StartNetwork ...
func (c *NetworksController) StartNetwork(cert *pb.Certificate) (*NetworkServices, error) {
	network, err := c.networks.AddNetwork(cert)
	if err != nil {
		return nil, err
	}

	hashTable := vpn.NewHashTable(c.logger, network, c.hashTableStore)
	peerIndex := vpn.NewPeerIndex(c.logger, network, c.peerIndexStore)
	peerExchange := vpn.NewPeerExchange(c.logger, network)

	network.SetHandler(vpn.HashTablePort, hashTable)
	network.SetHandler(vpn.PeerIndexPort, peerIndex)
	network.SetHandler(vpn.PeerExchangePort, peerExchange)

	svc := &NetworkServices{
		Host:         c.host,
		Network:      network,
		HashTable:    hashTable,
		PeerIndex:    peerIndex,
		PeerExchange: peerExchange,
		Swarms:       c.swarmController.AddNetwork(network),
	}
	c.networkServicesLock.Lock()
	c.networkServices.Insert(dao.GetRootCert(cert).Key, svc)
	c.networkServicesLock.Unlock()

	return svc, nil
}

// StartNetworkAuthorityServices ...
func (c *NetworksController) StartNetworkAuthorityServices(cert *pb.Certificate, options *pb.Network) error {
	c.networkServicesLock.Lock()
	svc, ok := c.networkServices.Get(dao.GetRootCert(cert).Key)
	c.networkServicesLock.Unlock()
	if !ok {
		return errors.New("unknown network")
	}

	_ = svc

	// TODO: some sort of replication?
	// hot standby?

	// d := NewDirectoryService(svc, options)
	// go d.Run()
	// svc.Network.SetHandler(vpn.DirectoryPort, d)

	return nil
}

// StopNetwork ...
func (c *NetworksController) StopNetwork(cert *pb.Certificate) error {
	c.networkServicesLock.Lock()
	svc, ok := c.networkServices.Get(dao.GetRootCert(cert).Key)
	c.networkServicesLock.Unlock()
	if !ok {
		return nil
	}

	return c.networks.RemoveNetwork(svc.Network)
}

// NetworkController ...
type NetworkController struct {
	host    *vpn.Host
	network *vpn.Network
	// store   *store
	pex *vpn.PeerExchange
}

type networkServicesMap struct {
	m llrb.LLRB
}

func (m *networkServicesMap) Insert(k []byte, v *NetworkServices) {
	m.m.InsertNoReplace(networkServicesMapEntry{k, v})
}

func (m *networkServicesMap) Delete(k []byte) {
	m.m.Delete(networkServicesMapEntry{k, nil})
}

func (m *networkServicesMap) Get(k []byte) (*NetworkServices, bool) {
	if it := m.m.Get(networkServicesMapEntry{k, nil}); it != nil {
		return it.(networkServicesMapEntry).v, true
	}
	return nil, false
}

type networkServicesMapEntry struct {
	k []byte
	v *NetworkServices
}

func (t networkServicesMapEntry) Less(oi llrb.Item) bool {
	if o, ok := oi.(networkServicesMapEntry); ok {
		return bytes.Compare(t.k, o.k) == -1
	}
	return !oi.Less(t)
}
