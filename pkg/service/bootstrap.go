package service

import (
	"errors"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// BootstrapServiceOptions ...
type BootstrapServiceOptions struct {
	EnablePublishing bool
}

// NewBootstrapService ...
func NewBootstrapService(
	logger *zap.Logger,
	host *vpn.Host,
	store *dao.ProfileStore,
	networkController *NetworkController,
	opt BootstrapServiceOptions,
) *BootstrapService {
	s := &BootstrapService{
		logger:            logger,
		store:             store,
		networkController: networkController,
		enablePublishing:  opt.EnablePublishing,
	}

	host.AddPeerHandler(s.handlePeer)

	return s
}

// BootstrapService ...
type BootstrapService struct {
	logger            *zap.Logger
	store             *dao.ProfileStore
	networkController *NetworkController
	peersLock         sync.Mutex
	peers             bootstrapServicePeerMap
	enablePublishing  bool
}

func (c *BootstrapService) handlePeer(peer *vpn.Peer) {
	c.logger.Debug("creating boostrap service peer", logutil.ByteHex("peer", peer.Certificate.Key))
	p := newBootstrapServicePeer(c.logger, c.store, c.networkController, peer)

	go func() {
		if c.enablePublishing {
			if err := p.EnablePublishing(); err != nil {
				c.logger.Info("error sending thing", zap.Error(err))
			}
		}

		c.peersLock.Lock()
		c.peers.Insert(peer.HostID(), p)
		c.peersLock.Unlock()

		<-peer.Done()

		c.peersLock.Lock()
		c.peers.Delete(peer.HostID())
		c.peersLock.Unlock()
	}()
}

// HandleFrame ...
func (c *BootstrapService) HandleFrame(p *vpn.Peer, f vpn.Frame) error {
	return nil
}

// GetPeerHostIDs ...
func (c *BootstrapService) GetPeerHostIDs() []kademlia.ID {
	c.peersLock.Lock()
	defer c.peersLock.Unlock()

	ids := []kademlia.ID{}
	c.peers.Each(func(p *bootstrapServicePeer) bool {
		if p.enablePublishing {
			ids = append(ids, p.vpnPeer.HostID())
		}
		return true
	})
	return ids
}

// PublishNetwork ...
func (c *BootstrapService) PublishNetwork(hostID kademlia.ID, network *pb.Network) error {
	c.peersLock.Lock()
	defer c.peersLock.Unlock()

	p, ok := c.peers.Get(hostID)
	if !ok {
		return errors.New("peer not found")
	}

	return p.PublishNetwork(network, time.Hour*24*365)
}

func newBootstrapServicePeer(
	logger *zap.Logger,
	store *dao.ProfileStore,
	networkController *NetworkController,
	peer *vpn.Peer,
) *bootstrapServicePeer {
	s := &bootstrapServicePeer{
		logger:            logger,
		vpnPeer:           peer,
		ch:                vpn.NewFrameReadWriter(peer.Link, vpn.BootstrapPort, peer.Link.MTU()),
		store:             store,
		networkController: networkController,
	}

	peer.SetHandler(vpn.BootstrapPort, s.ch.HandleFrame)
	go func() {
		s.readMessages()

		peer.RemoveHandler(vpn.BootstrapPort)
	}()

	return s
}

// bootstrapServicePeer ...
type bootstrapServicePeer struct {
	logger            *zap.Logger
	vpnPeer           *vpn.Peer
	ch                *vpn.FrameReadWriter
	store             *dao.ProfileStore
	networkController *NetworkController
	enablePublishing  bool
}

func (s *bootstrapServicePeer) readMessages() {
	var msg pb.BootstrapServiceMessage
	for {
		if err := vpn.ReadProtoStream(s.ch, &msg); err != nil {
			s.logger.Info("bootstrap service peer read error", zap.Error(err))
			break
		}

		switch b := msg.Body.(type) {
		case *pb.BootstrapServiceMessage_BrokerOffer_:
			_ = b
			s.logger.Info("bootstrap offer received", logutil.ByteHex("peer", s.vpnPeer.Certificate.Key))
			s.enablePublishing = true
		case *pb.BootstrapServiceMessage_PublishRequest_:
			if err := s.handlePublish(b.PublishRequest); err != nil {
				s.logger.Info("bootstrap service publish error", zap.Error(err))
			}
		}
	}
}

func (s *bootstrapServicePeer) handlePublish(r *pb.BootstrapServiceMessage_PublishRequest) error {
	membership, err := dao.NewNetworkMembershipFromCertificate(r.Name, r.Certificate)
	if err != nil {
		return err
	}
	err = dao.InsertNetworkMembership(s.store, membership)
	if err != nil {
		return err
	}

	_, err = s.networkController.StartNetwork(r.Certificate, WithMemberServices(s.logger))
	if err != nil {
		return err
	}

	return nil
}

// PublishNetwork ...
func (s *bootstrapServicePeer) PublishNetwork(network *pb.Network, validDuration time.Duration) error {
	csr := &pb.CertificateRequest{
		Key:      s.vpnPeer.Certificate.Key,
		KeyType:  s.vpnPeer.Certificate.KeyType,
		KeyUsage: uint32(pb.KeyUsage_KEY_USAGE_BROKER),
	}

	cert, err := dao.SignCertificateRequest(csr, validDuration, network.Key)
	if err != nil {
		return err
	}
	cert.ParentOneof = &pb.Certificate_Parent{Parent: network.Certificate}

	err = vpn.WriteProtoStream(s.ch, &pb.BootstrapServiceMessage{
		Body: &pb.BootstrapServiceMessage_PublishRequest_{
			PublishRequest: &pb.BootstrapServiceMessage_PublishRequest{
				Name:        network.Name,
				Certificate: cert,
			},
		},
	})
	if err != nil {
		return err
	}
	return s.ch.Flush()
}

// EnablePublishing ...
func (s *bootstrapServicePeer) EnablePublishing() error {
	err := vpn.WriteProtoStream(s.ch, &pb.BootstrapServiceMessage{
		Body: &pb.BootstrapServiceMessage_BrokerOffer_{
			BrokerOffer: &pb.BootstrapServiceMessage_BrokerOffer{},
		},
	})
	if err != nil {
		return err
	}
	return s.ch.Flush()
}

type bootstrapServicePeerMap struct {
	m llrb.LLRB
}

func (m *bootstrapServicePeerMap) Insert(k kademlia.ID, v *bootstrapServicePeer) {
	m.m.InsertNoReplace(bootstrapServicePeerMapItem{k, v})
}

func (m *bootstrapServicePeerMap) Delete(k kademlia.ID) {
	m.m.Delete(bootstrapServicePeerMapItem{k, nil})
}

func (m *bootstrapServicePeerMap) Get(k kademlia.ID) (*bootstrapServicePeer, bool) {
	if it := m.m.Get(bootstrapServicePeerMapItem{k, nil}); it != nil {
		return it.(bootstrapServicePeerMapItem).v, true
	}
	return nil, false
}

func (m *bootstrapServicePeerMap) Each(f func(b *bootstrapServicePeer) bool) {
	m.m.AscendGreaterOrEqual(llrb.Inf(-1), func(i llrb.Item) bool {
		return f(i.(bootstrapServicePeerMapItem).v)
	})
}

type bootstrapServicePeerMapItem struct {
	k kademlia.ID
	v *bootstrapServicePeer
}

func (t bootstrapServicePeerMapItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(bootstrapServicePeerMapItem); ok {
		return t.k.Less(o.k)
	}
	return !oi.Less(t)
}

// StartBootstrapClients ...
func StartBootstrapClients(host *vpn.Host, store *dao.ProfileStore) error {
	clients, err := dao.GetBootstrapClients(store)
	if err != nil {
		return err
	}

	for _, o := range clients {
		switch o := o.ClientOptions.(type) {
		case *pb.BootstrapClient_WebsocketOptions:
			go func() {
				if err := host.Dial(vpn.WebSocketAddr(o.WebsocketOptions.Url)); err != nil {
					log.Println(err)
				}
			}()
		}
	}
	return nil
}
