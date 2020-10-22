package bootstrap

import (
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// ServiceOptions ...
type ServiceOptions struct {
	EnablePublishing bool
}

// NewService ...
func NewService(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	opt ServiceOptions,
) *Service {
	s := &Service{
		logger:           logger,
		store:            store,
		vpn:              vpn,
		enablePublishing: opt.EnablePublishing,
	}

	vpn.VNIC().AddPeerHandler(s.handlePeer)

	return s
}

// Service ...
type Service struct {
	logger           *zap.Logger
	store            *dao.ProfileStore
	vpn              *vpn.Host
	peersLock        sync.Mutex
	peers            peerMap
	enablePublishing bool
}

func (c *Service) handlePeer(peer *vnic.Peer) {
	c.logger.Debug("creating boostrap service peer", logutil.ByteHex("peer", peer.Certificate.Key))
	p := newPeer(c.logger, c.store, c.vpn, peer)

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
func (c *Service) HandleFrame(p *vnic.Peer, f vnic.Frame) error {
	return nil
}

// GetPeerHostIDs ...
func (c *Service) GetPeerHostIDs() []kademlia.ID {
	c.peersLock.Lock()
	defer c.peersLock.Unlock()

	ids := []kademlia.ID{}
	c.peers.Each(func(p *peer) bool {
		if p.enablePublishing {
			ids = append(ids, p.vnicPeer.HostID())
		}
		return true
	})
	return ids
}

// PublishNetwork ...
func (c *Service) PublishNetwork(hostID kademlia.ID, network *pb.Network) error {
	c.peersLock.Lock()
	defer c.peersLock.Unlock()

	p, ok := c.peers.Get(hostID)
	if !ok {
		return errors.New("peer not found")
	}

	return p.PublishNetwork(network, time.Hour*24*365)
}

// Dial ...
func (c *Service) Dial(client *pb.BootstrapClient) {
	switch client := client.ClientOptions.(type) {
	case *pb.BootstrapClient_WebsocketOptions:
		go func() {
			err := c.vpn.VNIC().Dial(vnic.WebSocketAddr{
				URL:                   client.WebsocketOptions.Url,
				InsecureSkipVerifyTLS: client.WebsocketOptions.InsecureSkipVerifyTls,
			})
			if err != nil {
				c.logger.Debug("websocket botstrap client dial failed", zap.Error(err))
			}
		}()
	}
}

func newPeer(
	logger *zap.Logger,
	store *dao.ProfileStore,
	vpn *vpn.Host,
	vnicPeer *vnic.Peer,
) *peer {
	s := &peer{
		logger:   logger,
		vnicPeer: vnicPeer,
		ch:       vnicPeer.Channel(vnic.BootstrapPort),
		store:    store,
		vpn:      vpn,
	}

	go s.readMessages()

	return s
}

// peer ...
type peer struct {
	logger           *zap.Logger
	vnicPeer         *vnic.Peer
	ch               *vnic.FrameReadWriter
	store            *dao.ProfileStore
	vpn              *vpn.Host
	enablePublishing bool
}

func (s *peer) readMessages() {
	var msg pb.BootstrapServiceMessage
	for {
		if err := protoutil.ReadStream(s.ch, &msg); err != nil {
			s.logger.Info("bootstrap service peer read error", zap.Error(err))
			break
		}

		switch b := msg.Body.(type) {
		case *pb.BootstrapServiceMessage_BrokerOffer_:
			_ = b
			s.logger.Info("bootstrap offer received", logutil.ByteHex("peer", s.vnicPeer.Certificate.Key))
			s.enablePublishing = true
		case *pb.BootstrapServiceMessage_PublishRequest_:
			if err := s.handlePublish(b.PublishRequest); err != nil {
				s.logger.Info("bootstrap service publish error", zap.Error(err))
			}
		}
	}
}

func (s *peer) handlePublish(r *pb.BootstrapServiceMessage_PublishRequest) error {
	network, err := dao.NewNetworkFromCertificate(r.Certificate)
	if err != nil {
		return err
	}
	err = dao.UpsertNetwork(s.store, network)
	if err != nil {
		return err
	}

	if _, err = s.vpn.AddNetwork(r.Certificate); err != nil {
		return err
	}

	return nil
}

// PublishNetwork ...
func (s *peer) PublishNetwork(network *pb.Network, validDuration time.Duration) error {
	networkCert, err := dao.NewNetworkCertificate(network)
	if err != nil {
		return err
	}
	csr := &pb.CertificateRequest{
		Key:      s.vnicPeer.Certificate.Key,
		KeyType:  s.vnicPeer.Certificate.KeyType,
		KeyUsage: uint32(pb.KeyUsage_KEY_USAGE_BROKER),
	}
	cert, err := dao.SignCertificateRequest(csr, validDuration, network.Key)
	if err != nil {
		return err
	}
	cert.ParentOneof = &pb.Certificate_Parent{Parent: networkCert}

	err = protoutil.WriteStream(s.ch, &pb.BootstrapServiceMessage{
		Body: &pb.BootstrapServiceMessage_PublishRequest_{
			PublishRequest: &pb.BootstrapServiceMessage_PublishRequest{
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
func (s *peer) EnablePublishing() error {
	err := protoutil.WriteStream(s.ch, &pb.BootstrapServiceMessage{
		Body: &pb.BootstrapServiceMessage_BrokerOffer_{
			BrokerOffer: &pb.BootstrapServiceMessage_BrokerOffer{},
		},
	})
	if err != nil {
		return err
	}
	return s.ch.Flush()
}

type peerMap struct {
	m llrb.LLRB
}

func (m *peerMap) Insert(k kademlia.ID, v *peer) {
	m.m.InsertNoReplace(peerMapItem{k, v})
}

func (m *peerMap) Delete(k kademlia.ID) {
	m.m.Delete(peerMapItem{k, nil})
}

func (m *peerMap) Get(k kademlia.ID) (*peer, bool) {
	if it := m.m.Get(peerMapItem{k, nil}); it != nil {
		return it.(peerMapItem).v, true
	}
	return nil, false
}

func (m *peerMap) Each(f func(b *peer) bool) {
	m.m.AscendGreaterOrEqual(llrb.Inf(-1), func(i llrb.Item) bool {
		return f(i.(peerMapItem).v)
	})
}

type peerMapItem struct {
	k kademlia.ID
	v *peer
}

func (t peerMapItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(peerMapItem); ok {
		return t.k.Less(o.k)
	}
	return !oi.Less(t)
}
