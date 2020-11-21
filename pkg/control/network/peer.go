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
		peerInit:   make(chan uint32, 1),
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

	lock       sync.Mutex
	links      map[uint64]*networkBinding
	syncing    int32
	peerInit   chan uint32
	bindings   chan []*pb.NetworkPeerBinding
	brokerConn *vnic.FrameReadWriter
}

// SetPeerInit ...
func (p *Peer) SetPeerInit(keyCount uint32) {
	select {
	case p.peerInit <- keyCount:
	default:
	}
}

// SetPeerBindings ...
func (p *Peer) SetPeerBindings(bindings []*pb.NetworkPeerBinding) {
	select {
	case p.bindings <- bindings:
	default:
	}
}

func (p *Peer) closeNetwork(networkKey []byte) {
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

	p.client.Network().Close(context.Background(), &pb.NetworkPeerCloseRequest{Key: networkKey}, &pb.NetworkPeerCloseResponse{})
}

func (p *Peer) sync(ctx context.Context) error {
	if !atomic.CompareAndSwapInt32(&p.syncing, 0, 1) {
		return errors.New("already syncing")
	}
	defer atomic.StoreInt32(&p.syncing, 0)

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
	case keyCount := <-p.peerInit:
		if len(keys) == 0 || keyCount == 0 {
			return nil
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
			return ErrNetworkOwnerMismatch
		}
		if !bytes.Equal(networkKeyForCertificate(binding.Certificate), networkKey) {
			return ErrNetworkAuthorityMismatch
		}
		if peerBinding.Port > uint32(math.MaxUint16) {
			return ErrNetworkIDBounds
		}

		link := &networkBinding{
			localPort: uint16(binding.Port),
			peerPort:  uint16(peerBinding.Port),
		}

		c, ok := p.certificates.Get(networkKey)
		if !ok {
			continue
		}
		p.links[c.networkID] = link

		if !isCertificateTrusted(binding.Certificate) || !isCertificateTrusted(peerBinding.Certificate) {
			continue
		}

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
		link.open = true

		p.observers.Network.Emit(event.NetworkPeerOpen{PeerID: p.id, NetworkKey: networkKey})
	}
	return nil
}

type networkBinding struct {
	localPort uint16
	peerPort  uint16
	open      bool
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
