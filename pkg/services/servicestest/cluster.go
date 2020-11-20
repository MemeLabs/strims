package servicestest

import (
	"fmt"
	"sort"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/memkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewNode ...
func NewNode(logger *zap.Logger, i int) (*Node, error) {
	profile, err := dao.NewProfile(fmt.Sprintf("user %d", i))
	if err != nil {
		return nil, err
	}

	blobStore, err := memkv.NewStore("test")
	if err != nil {
		return nil, err
	}
	storageKey, err := dao.NewStorageKey("test")
	if err != nil {
		return nil, err
	}
	profileStore := dao.NewProfileStore(profile.Id, blobStore, storageKey)
	if err := profileStore.Init(profile); err != nil {
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

	return &Node{
		ID:      id,
		Store:   profileStore,
		Profile: profile,
		VNIC:    vnicHost,
		VPN:     vpnHost,
		peers:   map[string]struct{}{},
	}, nil
}

// Node ...
type Node struct {
	Store   *dao.ProfileStore
	Profile *pb.Profile
	ID      kademlia.ID
	VNIC    *vnic.Host
	VPN     *vpn.Host
	Client  *vpn.Client

	peers map[string]struct{}
	conns []*ppspptest.MeterConn
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
	Logger       *zap.Logger
	NodeCount    int
	PeersPerNode int
	Network      *pb.Network
	Nodes        []*Node
}

// Run ...
func (c *Cluster) Run() error {
	for i := 0; i < c.NodeCount; i++ {
		n, err := NewNode(c.Logger, i)
		if err != nil {
			return err
		}
		c.Nodes = append(c.Nodes, n)
	}

	var err error
	c.Network, err = dao.NewNetwork("network", nil, c.Nodes[0].Profile)
	if err != nil {
		return err
	}

	c.Nodes[0].Client, err = c.Nodes[0].VPN.AddNetwork(c.Network.Certificate)
	if err != nil {
		return err
	}

	for _, node := range c.Nodes[1:] {
		csr, err := dao.NewCertificateRequest(
			node.Profile.Key,
			pb.KeyUsage_KEY_USAGE_PEER|pb.KeyUsage_KEY_USAGE_SIGN,
			dao.WithSubject(c.Nodes[0].Profile.Name),
		)
		if err != nil {
			return err
		}

		cert, err := dao.SignCertificateRequest(csr, time.Hour*24, c.Network.Key)
		if err != nil {
			return err
		}
		cert.ParentOneof = &pb.Certificate_Parent{Parent: c.Network.Certificate}

		node.Client, err = node.VPN.AddNetwork(cert)
		if err != nil {
			return err
		}
	}

	wg := sync.WaitGroup{}
	for _, node := range c.Nodes {
		node.VNIC.AddPeerHandler(func(p *vnic.Peer) {
			wg.Done()
		})
	}

	// init node links
	sortedNodes := make([]*Node, len(c.Nodes))
	copy(sortedNodes, c.Nodes)

	for i := 0; i < len(c.Nodes); i++ {
		sort.Sort(&nodesByXOrDistance{c.Nodes[i].VNIC.ID(), sortedNodes})

		n := 0
		off := 1
		k := c.PeersPerNode / 2
		for n < c.PeersPerNode && k != 0 {
			for j := off; j < off+k && j < len(sortedNodes); j++ {
				if _, ok := c.Nodes[i].peers[sortedNodes[j].VNIC.ID().String()]; ok {
					continue
				}
				c.Nodes[i].peers[sortedNodes[j].VNIC.ID().String()] = struct{}{}
				sortedNodes[j].peers[c.Nodes[i].VNIC.ID().String()] = struct{}{}

				c0, c1 := ppspptest.NewUnbufferedConnPair()

				mc0 := ppspptest.NewMeterConn(c0)
				mc1 := ppspptest.NewMeterConn(c1)

				c.Nodes[i].conns = append(c.Nodes[i].conns, mc0)
				sortedNodes[j].conns = append(sortedNodes[j].conns, mc1)

				wg.Add(2)
				c.Nodes[i].VNIC.AddLink(mc0)
				sortedNodes[j].VNIC.AddLink(mc1)

				n++
			}

			off = (off + k) * 2
			k /= 2
		}
	}

	wg.Wait()

	// init node network links
	for _, node := range c.Nodes {
		for _, peer := range node.VNIC.Peers() {
			node.Client.Network.AddPeer(peer, 10000, 10000)
		}
	}

	return nil
}

// MessageHandlerFunc ...
func MessageHandlerFunc(fn func(*vpn.Message) (bool, error)) vpn.MessageHandler {
	return &messageHandler{fn}
}

type messageHandler struct {
	handleMessage func(*vpn.Message) (bool, error)
}

func (h *messageHandler) HandleMessage(msg *vpn.Message) (bool, error) {
	return h.handleMessage(msg)
}

type nodesByXOrDistance struct {
	id    kademlia.ID
	nodes []*Node
}

func (s nodesByXOrDistance) Len() int {
	return len(s.nodes)
}

func (s nodesByXOrDistance) Swap(i, j int) {
	s.nodes[i], s.nodes[j] = s.nodes[j], s.nodes[i]
}

func (s nodesByXOrDistance) Less(i, j int) bool {
	return s.id.XOr(s.nodes[i].ID).Less(s.id.XOr(s.nodes[j].ID))
}
