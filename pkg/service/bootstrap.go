package service

import (
	"bytes"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

const bootstrapServicePort = 3

type BootstrapServiceOptions struct {
	EnablePublishing bool
}

// NewBootstrapService ...
func NewBootstrapService(
	logger *zap.Logger,
	store *dao.ProfileStore,
	networksController *NetworksController,
	opt BootstrapServiceOptions,
) *BootstrapService {
	return &BootstrapService{
		logger:             logger,
		store:              store,
		networksController: networksController,
		enablePublishing:   opt.EnablePublishing,
	}
}

// BootstrapService ...
type BootstrapService struct {
	logger             *zap.Logger
	store              *dao.ProfileStore
	networksController *NetworksController
	peersLock          sync.Mutex
	peers              bootstrapServicePeerMap
	enablePublishing   bool
}

func (c *BootstrapService) handleHost(h *vpn.Host) error {
	go func() {
		ch := make(chan *vpn.Peer)
		h.NotifyPeer(ch)
		for peer := range ch {
			go c.handlePeer(peer)
		}
	}()

	return nil
}

func (c *BootstrapService) handlePeer(peer *vpn.Peer) {
	p := newBootstrapServicePeer(c.logger, c.store, c.networksController, peer)
	if c.enablePublishing {
		if err := p.EnablePublishing(); err != nil {
			c.logger.Info("error sending thing", zap.Error(err))
		}
	}
	c.peersLock.Lock()
	c.peers.Insert(peer.Certificate.Key, p)
	c.peersLock.Unlock()

	<-peer.Done()

	c.peersLock.Lock()
	c.peers.Delete(peer.Certificate.Key)
	c.peersLock.Unlock()
}

// HandleFrame ...
func (c *BootstrapService) HandleFrame(p *vpn.Peer, f vpn.Frame) error {
	return nil
}

// GetPeerKeys ...
func (c *BootstrapService) GetPeerKeys() [][]byte {
	c.peersLock.Lock()
	defer c.peersLock.Unlock()

	keys := [][]byte{}
	c.peers.Each(func(p *bootstrapServicePeer) bool {
		if p.enablePublishing {
			keys = append(keys, p.vpnPeer.Certificate.Key)
		}
		return true
	})
	return keys
}

// PublishNetwork ...
func (c *BootstrapService) PublishNetwork(peerKey []byte, network *pb.Network) error {
	c.peersLock.Lock()
	defer c.peersLock.Unlock()

	p, ok := c.peers.Get(peerKey)
	if !ok {
		return errors.New("peer not found")
	}

	return p.PublishNetwork(network, time.Hour*24*365)
}

func newBootstrapServicePeer(
	logger *zap.Logger,
	store *dao.ProfileStore,
	networksController *NetworksController,
	peer *vpn.Peer,
) *bootstrapServicePeer {
	s := &bootstrapServicePeer{
		logger:             logger,
		vpnPeer:            peer,
		ch:                 vpn.NewFrameReadWriter(peer.Link, bootstrapServicePort, peer.Link.MTU()),
		store:              store,
		networksController: networksController,
	}

	go func() {
		peer.SetHandler(bootstrapServicePort, s.ch.HandleFrame)
		s.doMemes()
		s.vpnPeer.RemoveHandler(bootstrapServicePort)
	}()

	return s
}

// bootstrapServicePeer ...
type bootstrapServicePeer struct {
	logger             *zap.Logger
	vpnPeer            *vpn.Peer
	ch                 *vpn.FrameReadWriter
	store              *dao.ProfileStore
	networksController *NetworksController
	enablePublishing   bool
}

func (s *bootstrapServicePeer) doMemes() {
	var msg pb.BootstrapServiceMessage
	for {
		if err := vpn.ReadProtoStream(s.ch, &msg); err != nil {
			s.logger.Info("bootstrap service peer read error", zap.Error(err))
			break
		}

		s.logger.Info("bootstrap peer thing got message")

		switch b := msg.Body.(type) {
		case *pb.BootstrapServiceMessage_BrokerOffer_:
			_ = b
			s.logger.Info("offer received")
			s.enablePublishing = true
		case *pb.BootstrapServiceMessage_PublishRequest_:
			s.handlePublish(b.PublishRequest)
		}
	}
}

func (s *bootstrapServicePeer) handlePublish(r *pb.BootstrapServiceMessage_PublishRequest) error {
	membership, err := dao.NewNetworkMembershipFromCertificate(r.Name, r.Certificate)
	if err != nil {
		return err
	}
	err = s.store.InsertNetworkMembership(membership)
	if err != nil {
		return err
	}

	_, err = s.networksController.StartNetwork(r.Certificate)
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

func (m *bootstrapServicePeerMap) Each(f func(b *bootstrapServicePeer) bool) {
	m.m.AscendGreaterOrEqual(llrb.Inf(-1), func(i llrb.Item) bool {
		return f(i.(bootstrapServicePeerMapEntry).v)
	})
}

func (m *bootstrapServicePeerMap) Insert(k []byte, v *bootstrapServicePeer) {
	m.m.InsertNoReplace(bootstrapServicePeerMapEntry{k, v})
}

func (m *bootstrapServicePeerMap) Delete(k []byte) {
	m.m.Delete(bootstrapServicePeerMapEntry{k, nil})
}

func (m *bootstrapServicePeerMap) Get(k []byte) (*bootstrapServicePeer, bool) {
	if it := m.m.Get(bootstrapServicePeerMapEntry{k, nil}); it != nil {
		return it.(bootstrapServicePeerMapEntry).v, true
	}
	return nil, false
}

type bootstrapServicePeerMapEntry struct {
	k []byte
	v *bootstrapServicePeer
}

func (t bootstrapServicePeerMapEntry) Less(oi llrb.Item) bool {
	if o, ok := oi.(bootstrapServicePeerMapEntry); ok {
		return bytes.Compare(t.k, o.k) == -1
	}
	return !oi.Less(t)
}

// WithBootstrapService ...
func WithBootstrapService(s *BootstrapService) vpn.HostOption {
	return s.handleHost
}

// WithBootstrapClients ...
func WithBootstrapClients(clients []*pb.BootstrapClient) vpn.HostOption {
	return func(host *vpn.Host) error {
		for _, o := range clients {
			switch o := o.ClientOptions.(type) {
			case *pb.BootstrapClient_WebsocketOptions:
				go host.Dial(vpn.WebSocketAddr(o.WebsocketOptions.Url))
			}
		}
		return nil
	}
}