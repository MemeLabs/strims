package network

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"math"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// PeerClient ..
type PeerClient interface {
	Bootstrap() *api.BootstrapPeerClient
	Network() *api.NetworkPeerClient
	CA() *api.CAPeerClient
}

// NewPeer ...
func NewPeer(
	id uint64,
	peer *vnic.Peer,
	client PeerClient,
	logger *zap.Logger,
	observers *event.Observers,
	broker Broker,
	vpn *vpn.Host,
	certificates *certificateMap,
) *Peer {
	return &Peer{
		id:           id,
		client:       client,
		peer:         peer,
		logger:       logger,
		observers:    observers,
		broker:       broker,
		vpn:          vpn,
		certificates: certificates,

		links:      make(map[uint64]*networkBinding),
		keyCount:   make(chan uint32, 1),
		bindings:   make(chan []*pb.NetworkPeerBinding, 1),
		brokerConn: peer.Channel(vnic.NetworkBrokerPort),
	}
}

// Peer ...
type Peer struct {
	id           uint64
	peer         *vnic.Peer
	client       PeerClient
	logger       *zap.Logger
	observers    *event.Observers
	broker       Broker
	vpn          *vpn.Host
	certificates *certificateMap

	lock        sync.Mutex
	links       map[uint64]*networkBinding
	negotiating uint32
	keyCount    chan uint32
	bindings    chan []*pb.NetworkPeerBinding
	brokerConn  *vnic.FrameReadWriter
}

// HandlePeerNegotiate ...
func (p *Peer) HandlePeerNegotiate(keyCount uint32) {
	select {
	case p.keyCount <- keyCount:
	default:
	}

	if atomic.LoadUint32(&p.negotiating) == 0 {
		go func() {
			if err := p.negotiateNetworks(context.Background()); err != nil {
				p.logger.Debug("network negotiation failed", zap.Error(err))
			}
		}()
	}
}

// HandlePeerOpen ...
func (p *Peer) HandlePeerOpen(bindings []*pb.NetworkPeerBinding) {
	select {
	case p.bindings <- bindings:
	default:
	}
}

// HandlePeerClose ...
func (p *Peer) HandlePeerClose(networkKey []byte) {
	p.closeNetworkWithoutNotifyingPeer(networkKey)
}

// HandlePeerUpdateCertificate ...
func (p *Peer) HandlePeerUpdateCertificate(cert *pb.Certificate) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	if err := dao.VerifyCertificate(cert); err != nil {
		p.logger.Debug("update certificate failed", zap.Error(err))
		return err
	}
	if !bytes.Equal(p.peer.Certificate.Key, cert.Key) {
		p.logger.Debug("update certificate failed", zap.Error(ErrCertificateOwnerMismatch))
		return ErrCertificateOwnerMismatch
	}
	if !isCertificateTrusted(cert) {
		p.logger.Debug("update certificate failed", zap.Error(ErrProvisionalCertificate))
		return ErrProvisionalCertificate
	}

	networkKey := networkKeyForCertificate(cert)
	c, ok := p.certificates.Get(networkKey)
	if !ok {
		p.logger.Debug("update certificate failed", zap.Error(ErrNetworkNotFound))
		return ErrNetworkNotFound
	}
	link, ok := p.links[c.networkID]
	if !ok {
		p.logger.Debug("update certificate failed", zap.Error(ErrNetworkBindingNotFound))
		return ErrNetworkBindingNotFound
	}

	link.peerCertTrusted = true
	return p.addNetwork(networkKey, link)
}

func (p *Peer) sendCertificateUpdate(network *pb.Network) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	link, ok := p.links[network.Id]
	if !ok {
		return ErrNetworkBindingNotFound
	}
	link.localCertTrusted = true

	err := p.client.Network().UpdateCertificate(
		context.Background(),
		&pb.NetworkPeerUpdateCertificateRequest{Certificate: network.Certificate},
		&pb.NetworkPeerUpdateCertificateResponse{},
	)
	if err != nil {
		return err
	}

	return p.addNetwork(networkKeyForCertificate(network.Certificate), link)
}

func (p *Peer) closeNetworkWithoutNotifyingPeer(networkKey []byte) {
	p.lock.Lock()
	defer p.lock.Unlock()

	c, ok := p.certificates.Get(networkKey)
	if !ok {
		return
	}
	if _, ok := p.links[c.networkID]; !ok {
		return
	}
	delete(p.links, c.networkID)

	if len(p.links) == 0 {
		p.peer.Close()
		return
	}

	client, ok := p.vpn.Client(networkKey)
	if !ok {
		return
	}
	client.Network.RemovePeer(p.peer.HostID())
}

func (p *Peer) closeNetwork(networkKey []byte) {
	p.closeNetworkWithoutNotifyingPeer(networkKey)
	p.client.Network().Close(context.Background(), &pb.NetworkPeerCloseRequest{Key: networkKey}, &pb.NetworkPeerCloseResponse{})
}

func (p *Peer) hasNetworkBinding(networkID uint64) bool {
	p.lock.Lock()
	defer p.lock.Unlock()
	return p.links[networkID] != nil
}

func (p *Peer) negotiateNetworks(ctx context.Context) error {
	if !atomic.CompareAndSwapUint32(&p.negotiating, 0, 1) {
		return errors.New("already syncing")
	}
	defer atomic.StoreUint32(&p.negotiating, 0)

	ctx, cancel := context.WithTimeout(ctx, time.Second*10)
	defer cancel()

	keys := p.certificates.Keys()
	err := p.client.Network().Negotiate(ctx, &pb.NetworkPeerNegotiateRequest{KeyCount: uint32(len(keys))}, &pb.NetworkPeerNegotiateResponse{})
	if err != nil {
		return fmt.Errorf("sending network negotiation init failed: %w", err)
	}

	return p.exchangeBindings(ctx, keys)
}

func (p *Peer) exchangeBindings(ctx context.Context, keys [][]byte) error {
	select {
	case <-ctx.Done():
		return fmt.Errorf("no network negotiation init received: %w", ctx.Err())
	case keyCount := <-p.keyCount:
		if len(keys) == 0 || keyCount == 0 {
			return errors.New("one or both peers have zero keys")
		}

		preferSend := p.peer.HostID().Less(p.vpn.VNIC().ID())
		if len(keys) > int(keyCount) || (len(keys) == int(keyCount) && preferSend) {
			return p.exchangeBindingsAsSender(ctx, keys)
		} else {
			return p.exchangeBindingsAsReceiver(ctx, keys)
		}
	}
}

func (p *Peer) exchangeBindingsAsReceiver(ctx context.Context, keys [][]byte) error {
	keys, err := p.broker.ReceiveKeys(p.brokerConn, keys)
	if err != nil {
		return fmt.Errorf("network key broker failed: %w", err)
	}
	networkBindings, err := p.sendNetworkBindings(ctx, keys)
	if err != nil {
		return err
	}

	select {
	case <-ctx.Done():
		return fmt.Errorf("peer network bindings not received: %w", ctx.Err())
	case peerNetworkBindings := <-p.bindings:
		if _, err = p.verifyNetworkBindings(peerNetworkBindings); err != nil {
			return err
		}

		p.observers.Network.Emit(event.NetworkPeerBindings{PeerID: p.id, NetworkKeys: keys})

		return p.handleNetworkBindings(networkBindings, peerNetworkBindings)
	}
}

func (p *Peer) exchangeBindingsAsSender(ctx context.Context, keys [][]byte) error {
	if err := p.broker.SendKeys(p.brokerConn, keys); err != nil {
		return fmt.Errorf("network key broker failed: %w", err)
	}

	select {
	case <-ctx.Done():
		return fmt.Errorf("peer network bindings not received: %w", ctx.Err())
	case peerNetworkBindings := <-p.bindings:
		keys, err := p.verifyNetworkBindings(peerNetworkBindings)
		if err != nil {
			return err
		}

		p.observers.Network.Emit(event.NetworkPeerBindings{PeerID: p.id, NetworkKeys: keys})

		networkBindings, err := p.sendNetworkBindings(ctx, keys)
		if err != nil {
			return err
		}

		return p.handleNetworkBindings(networkBindings, peerNetworkBindings)
	}
}

func (p *Peer) sendNetworkBindings(ctx context.Context, keys [][]byte) ([]*pb.NetworkPeerBinding, error) {
	var bindings []*pb.NetworkPeerBinding

	for _, key := range keys {
		c, ok := p.certificates.Get(key)
		if !ok {
			return nil, fmt.Errorf("certificate not found: %w", ErrNetworkNotFound)
		}

		if _, ok := p.links[c.networkID]; ok {
			continue
		}

		port, err := p.peer.ReservePort()
		if err != nil {
			return nil, err
		}

		bindings = append(
			bindings,
			&pb.NetworkPeerBinding{
				Port:        uint32(port),
				Certificate: c.certificate,
			},
		)
	}

	err := p.client.Network().Open(ctx, &pb.NetworkPeerOpenRequest{Bindings: bindings}, &pb.NetworkPeerOpenResponse{})
	if err != nil {
		return nil, err
	}
	return bindings, nil
}

func (p *Peer) verifyNetworkBindings(bindings []*pb.NetworkPeerBinding) ([][]byte, error) {
	if bindings == nil {
		return nil, ErrNetworkBindingsEmpty
	}

	keys := make([][]byte, len(bindings))
	for i, b := range bindings {
		if err, ok := dao.VerifyCertificate(b.Certificate).(dao.Errors); ok && !err.IncludesOnly(dao.ErrNotAfterRange) {
			return nil, err
		}
		keys[i] = networkKeyForCertificate(b.Certificate)
	}
	return keys, nil
}

func (p *Peer) handleNetworkBindings(networkBindings, peerNetworkBindings []*pb.NetworkPeerBinding) error {
	for i, peerBinding := range peerNetworkBindings {
		binding := networkBindings[i]
		networkKey := networkKeyForCertificate(peerBinding.Certificate)

		if !bytes.Equal(p.peer.Certificate.Key, peerBinding.Certificate.Key) {
			return ErrCertificateOwnerMismatch
		}
		if !bytes.Equal(networkKeyForCertificate(binding.Certificate), networkKey) {
			return ErrNetworkAuthorityMismatch
		}
		if peerBinding.Port > uint32(math.MaxUint16) {
			return ErrNetworkIDBounds
		}

		link := &networkBinding{
			localPort:        uint16(binding.Port),
			peerPort:         uint16(peerBinding.Port),
			localCertTrusted: isCertificateTrusted(binding.Certificate),
			peerCertTrusted:  isCertificateTrusted(peerBinding.Certificate),
		}

		c, ok := p.certificates.Get(networkKey)
		if !ok {
			continue
		}
		p.links[c.networkID] = link

		if !isCertificateTrusted(binding.Certificate) || !isCertificateTrusted(peerBinding.Certificate) {
			continue
		}

		if err := p.addNetwork(networkKey, link); err != nil {
			return err
		}
	}
	return nil
}

func (p *Peer) addNetwork(networkKey []byte, link *networkBinding) error {
	if link.open || !link.localCertTrusted || !link.peerCertTrusted {
		return nil
	}
	link.open = true

	p.logger.Info(
		"adding peer to network",
		zap.Stringer("peer", p.peer.HostID()),
		logutil.ByteHex("network", networkKey),
		zap.Uint16("localPort", link.localPort),
		zap.Uint16("peerPort", link.peerPort),
	)

	client, ok := p.vpn.Client(networkKey)
	if !ok {
		return ErrNetworkNotFound
	}
	client.Network.AddPeer(p.peer, link.localPort, link.peerPort)

	p.observers.Network.Emit(event.NetworkPeerOpen{PeerID: p.id, NetworkKey: networkKey})
	return nil
}

type networkBinding struct {
	localPort        uint16
	peerPort         uint16
	localCertTrusted bool
	peerCertTrusted  bool
	open             bool
}

func isCertificateTrusted(cert *pb.Certificate) bool {
	return bytes.Equal(networkKeyForCertificate(cert), cert.GetParent().Key)
}

func networkKeyForCertificate(cert *pb.Certificate) []byte {
	return dao.GetRootCert(cert).Key
}

func nextCertificateRenewTime(network *pb.Network) time.Time {
	if isCertificateSubjectMismatched(network) {
		return time.Now()
	}
	return time.Unix(int64(network.Certificate.NotAfter), 0).Add(-certRenewScheduleAheadDuration)
}

func isCertificateSubjectMismatched(network *pb.Network) bool {
	return network.AltProfileName != "" && network.AltProfileName != network.Certificate.Subject
}
