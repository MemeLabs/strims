package service

import (
	"bytes"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

var nextServiceOptionID uint32

// NetworkServices ...
type NetworkServices struct {
	ID           uint64
	ProfileStore *dao.ProfileStore
	Network      *vpn.Network
	HashTable    vpn.HashTable
	PeerIndex    vpn.PeerIndex
	PeerExchange *vpn.PeerExchange
	Swarms       SwarmNetwork
}

// NetworkServiceConstructor ...
type NetworkServiceConstructor func(n *NetworkServices) Frontend

// NewNetworksController ...
func NewNetworksController(logger *zap.Logger, store *dao.ProfileStore) *NetworksController {
	return &NetworksController{
		logger:         logger,
		store:          store,
		services:       map[string]NetworkServiceConstructor{},
		serviceOptions: map[uint32]NetworkServices{},
	}
}

// NetworksController ...
type NetworksController struct {
	logger          *zap.Logger
	store           *dao.ProfileStore
	services        map[string]NetworkServiceConstructor
	serviceOptions  map[uint32]NetworkServices
	host            *vpn.Host
	networks        *vpn.Networks
	hashTableStore  *vpn.HashTableStore
	peerIndexStore  *vpn.PeerIndexStore
	swarmController *swarmController
}

// WithNetworkController ...
func WithNetworkController(c *NetworksController) vpn.HostOption {
	return func(h *vpn.Host) error {
		c.host = h
		c.networks = vpn.NewNetworks(h)
		c.hashTableStore = vpn.NewHashTableStore(h.ID())
		c.peerIndexStore = vpn.NewPeerIndexStore(h.ID())
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
		_, err := c.StartNetwork(m, pbNetworks(networks).Find(m.Certificate.GetParent().Key))
		if err != nil {
			c.logger.Error("failed to start network", zap.Error(err))
		}
	}

	return nil
}

// ServiceOptions ...
func (c *NetworksController) ServiceOptions(i uint32) NetworkServices {
	return c.serviceOptions[i]
}

// StartNetwork ...
func (c *NetworksController) StartNetwork(
	membership *pb.NetworkMembership,
	options *pb.Network,
) (*vpn.Network, error) {
	network, err := c.networks.AddNetwork(membership.Certificate)
	if err != nil {
		return nil, err
	}

	hashTable := vpn.NewHashTable(network, c.hashTableStore)
	peerIndex := vpn.NewPeerIndex(network, c.peerIndexStore)
	peerExchange := vpn.NewPeerExchange(network)

	network.SetHandler(vpn.HashTablePort, hashTable)
	network.SetHandler(vpn.PeerIndexPort, peerIndex)
	network.SetHandler(vpn.PeerExchangePort, peerExchange)

	opt := NetworkServices{
		Network:      network,
		HashTable:    hashTable,
		PeerIndex:    peerIndex,
		PeerExchange: peerExchange,
		Swarms:       c.swarmController.AddNetwork(network),
	}

	c.serviceOptions[atomic.AddUint32(&nextServiceOptionID, 1)] = opt

	if options != nil {
		c.startNetworkAuthorityServices(&opt, options)
	}

	return network, nil
}

func (c *NetworksController) startNetworkAuthorityServices(opt *NetworkServices, options *pb.Network) {
	// TODO: some sort of replication?
	// hot standby?

	d := NewDirectoryService(opt, options)

	// go d.Run()

	opt.Network.SetHandler(vpn.DirectoryPort, d)

}

// StopNetwork ...
func (c *NetworksController) StopNetwork(membership *pb.NetworkMembership) error {
	network, ok := c.networks.FindByKey(membership.Certificate.GetParent().Key)
	if !ok {
		return nil
	}
	return c.networks.RemoveNetwork(network)
}

func (c *NetworksController) temp() {
	// hostOptions = append(hostOptions, vpn.WithNetworks(networks))

	// chatServers, err := session.Store().GetChatServers()
	// if err != nil {
	// 	return nil, err
	// }
	// for _, s := range chatServers {
	// 	n, ok := networks.FindByKey(s.NetworkKey)
	// 	if !ok {
	// 		log.Printf("chat server %d bound to unknown network %x", s.Id, s.NetworkKey)
	// 		continue
	// 	}
	// 	if err := n.HostService(vpn.NewChatService(n, s.ChatRoom)); err != nil {
	// 		return nil, err
	// 	}
	// }
}

// NetworkController ...
type NetworkController struct {
	host    *vpn.Host
	network *vpn.Network
	// store   *store
	pex *vpn.PeerExchange
}

// func RunDefaultServices(host *vpn.Host, networks *vpn.Networks) {
// 	for _, n := range networks.Networks() {
// 		handleNetwork(host, n)
// 	}

// 	ch := make(chan *vpn.Network, 1)
// 	networks.NotifyNetwork(ch)
// 	for n := range ch {
// 		handleNetwork(host, n)
// 	}
// }

func handleNetwork(host *vpn.Host, n *vpn.Network) {
	// n.SetHandler(1, newStore(n))

	// pex := vpn.NewPeerExchange(n)
	// n.SetHandler(2, pex)
	// go func() {
	// 	for range time.NewTicker(time.Second * 10).C {
	// 		pex.RequestPeers()
	// 	}
	// }()
}

// // HostService ...
// func (n *Network) HostService(s NetworkService) error {
// 	port, err := n.ReservePort()
// 	if err != nil {
// 		return err
// 	}
// 	n.SetHandler(port, s)

// 	if as, ok := s.(AdvertiseableService); ok {
// 		n.ScheduleServiceAdvertisements(as, port)
// 	}

// 	return nil
// }

// // TODO: does this really belong here...? move to... ServiceManager? lifecycle?
// func (n *Network) ScheduleServiceAdvertisements(as AdvertiseableService, port uint16) {
// 	go func() {
// 		// TODO: build service advertisement message...
// 		// * host id
// 		// * port
// 		// * service name/description?
// 		// * mime type?
// 		// * service specific conn info?
// 		// * service "type"? what is this...? mime type? protocol type? proto uri?
// 		// * service version?
// 		// * service specific health/activity metrics? viewers/chatters/bitrate/resolution/???
// 		// * metadata ppspp stream uri?

// 		// b := make([]byte, 128)

// 		// for range time.NewTicker(time.Second * 5).C {
// 		// 	a := as.ServiceAdvertisement()
// 		// 	a.HostId = n.hostID.Bytes()
// 		// 	a.Port = uint32(port)

// 		// 	if err := n.Send(NewHostID(n.CAKey(), 0), 0, b); err != nil {
// 		// 		log.Println(err)
// 		// 	}
// 		// }

// 		// m.WriteTo(w io.Writer, hostID kademlia.ID, key *pb.Key)

// 	}()
// }

// type NetworkService interface {
// 	HandleMessage(m *Message) (bool, error)
// }

// type AdvertiseableService interface {
// 	ServiceAdvertisement() *pb.ServiceAdvertisement
// }
