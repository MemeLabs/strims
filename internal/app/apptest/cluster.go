package apptest

import (
	"fmt"
	"sort"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/dao/daotest"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/kv/kvtest"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewHost ...
func NewHost(logger *zap.Logger, i int) (*Host, error) {
	profile, err := dao.NewProfile(fmt.Sprintf("user %d", i))
	if err != nil {
		return nil, err
	}

	blobStore := kvtest.NewMemStore()
	storageKey, err := dao.NewStorageKey("test")
	if err != nil {
		return nil, err
	}
	profileStore := dao.NewProfileStore(profile.Id, storageKey, blobStore, nil)
	if err := profileStore.Init(); err != nil {
		return nil, err
	}

	vnicHost, err := vnic.New(logger, profile.Key)
	if err != nil {
		return nil, err
	}

	vpnHost, err := vpn.New(logger, vnicHost)
	if err != nil {
		return nil, err
	}

	id, err := kademlia.UnmarshalID(profile.Key.Public)
	if err != nil {
		return nil, err
	}

	return &Host{
		Store:     profileStore,
		Profile:   profile,
		VNIC:      vnicHost,
		VPN:       vpnHost,
		profileID: id,
		peers:     map[string]struct{}{},
	}, nil
}

// Host ...
type Host struct {
	Store    *dao.ProfileStore
	Profile  *profilev1.Profile
	VNIC     *vnic.Host
	VPN      *vpn.Host
	Node     *vpn.Node
	HostCert *certificate.Certificate
	Network  *networkv1.Network

	profileID kademlia.ID
	peers     map[string]struct{}
	conns     []ppspptest.Conn
}

// NewCluster ...
func NewCluster(logger *zap.Logger) (*Cluster, error) {
	c := &Cluster{
		Logger:       logger,
		NodeCount:    200,
		PeersPerNode: 15,
	}
	return c, c.Run()
}

// Cluster ...
type Cluster struct {
	Logger             *zap.Logger
	NodeCount          int
	PeersPerNode       int
	Hosts              []*Host
	SkipNetworkBinding bool
}

// Run ...
func (c *Cluster) Run() error {
	for i := 0; i < c.NodeCount; i++ {
		n, err := NewHost(c.Logger, i)
		if err != nil {
			return err
		}
		c.Hosts = append(c.Hosts, n)
	}

	network, err := dao.NewNetwork(&daotest.IDGenerator{}, "network", nil, c.Hosts[0].Profile)
	c.Hosts[0].Network = network
	if err != nil {
		return err
	}
	err = dao.Networks.Upsert(c.Hosts[0].Store, network)
	if err != nil {
		return err
	}

	for _, node := range c.Hosts[1:] {
		peerCert, err := createCert(node.Profile.Key, node.Profile.Name, network.GetServerConfig().Key, network.Certificate.GetParent())
		if err != nil {
			return err
		}
		hostCert, err := createCert(node.Profile.Key, node.Profile.Name, network.GetServerConfig().Key, network.Certificate.GetParent())
		if err != nil {
			return err
		}
		node.HostCert = hostCert

		node.Network, err = dao.NewNetworkFromCertificate(node.Store, peerCert)
		if err != nil {
			return err
		}
		err = dao.Networks.Upsert(node.Store, node.Network)
		if err != nil {
			return err
		}
	}

	wg := sync.WaitGroup{}
	for _, node := range c.Hosts {
		node.VNIC.AddPeerHandler(func(p *vnic.Peer) {
			wg.Done()
		})
	}

	// init node links
	sortedNodes := make([]*Host, len(c.Hosts))
	copy(sortedNodes, c.Hosts)

	for i := 0; i < len(c.Hosts); i++ {
		sort.Sort(&nodesByXOrDistance{c.Hosts[i].VNIC.ID(), sortedNodes})

		n := 0
		off := 1
		k := (c.PeersPerNode + 1) / 2
		for n < c.PeersPerNode && k != 0 {
			for j := off; j < off+k && j < len(sortedNodes); j++ {
				if _, ok := c.Hosts[i].peers[sortedNodes[j].VNIC.ID().String()]; ok {
					continue
				}
				c.Hosts[i].peers[sortedNodes[j].VNIC.ID().String()] = struct{}{}
				sortedNodes[j].peers[c.Hosts[i].VNIC.ID().String()] = struct{}{}

				c0, c1 := ppspptest.NewUnbufferedConnPair()

				c.Hosts[i].conns = append(c.Hosts[i].conns, c0)
				sortedNodes[j].conns = append(sortedNodes[j].conns, c1)

				wg.Add(2)
				c.Hosts[i].VNIC.AddLink(c0)
				sortedNodes[j].VNIC.AddLink(c1)

				n++
			}

			off = (off + k) * 2
			k /= 2
		}
	}

	wg.Wait()

	if c.SkipNetworkBinding {
		return nil
	}

	// init node network links
	for _, node := range c.Hosts {
		node.Node, err = node.VPN.AddNetwork(dao.NetworkKey(node.Network))
		if err != nil {
			return err
		}

		for _, peer := range node.VNIC.Peers() {
			node.Node.Network.AddPeer(peer, 10000, 10000)
		}
	}

	return nil
}

func createCert(key *key.Key, subject string, signingKey *key.Key, signingCert *certificate.Certificate) (*certificate.Certificate, error) {
	csr, err := dao.NewCertificateRequest(
		key,
		certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN,
		dao.WithSubject(subject),
	)
	if err != nil {
		return nil, err
	}

	cert, err := dao.SignCertificateRequest(csr, time.Hour*24*365, signingKey)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: signingCert}

	return cert, nil
}

// MessageHandlerFunc ...
func MessageHandlerFunc(fn func(*vpn.Message) error) vpn.MessageHandler {
	return &messageHandler{fn}
}

type messageHandler struct {
	handleMessage func(*vpn.Message) error
}

func (h *messageHandler) HandleMessage(msg *vpn.Message) error {
	return h.handleMessage(msg)
}

type nodesByXOrDistance struct {
	id    kademlia.ID
	nodes []*Host
}

func (s nodesByXOrDistance) Len() int {
	return len(s.nodes)
}

func (s nodesByXOrDistance) Swap(i, j int) {
	s.nodes[i], s.nodes[j] = s.nodes[j], s.nodes[i]
}

func (s nodesByXOrDistance) Less(i, j int) bool {
	return s.id.XOr(s.nodes[i].VNIC.ID()).Less(s.id.XOr(s.nodes[j].VNIC.ID()))
}
