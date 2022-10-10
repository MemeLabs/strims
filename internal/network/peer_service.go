// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package network

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"math"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

var _ networkv1.NetworkPeerService = (*peerService)(nil)

// NewPeer ...
func newPeer(
	id uint64,
	vnicPeer *vnic.Peer,
	client *networkv1.NetworkPeerClient,
	caClient *networkv1ca.CAPeerClient,
	logger *zap.Logger,
	observers *event.Observers,
	broker Broker,
	vpn *vpn.Host,
	qosc *qos.Class,
	certificates *certificateMap,
) *peerService {
	s := &peerService{
		id:       id,
		client:   client,
		caClient: caClient,
		vnicPeer: vnicPeer,
		logger: logger.With(
			zap.Uint64("id", id),
			logutil.ByteHex("host", vnicPeer.HostID().Bytes(nil)),
		),
		observers:    observers,
		broker:       broker,
		vpn:          vpn,
		certificates: certificates,

		keyCount:   make(chan uint32, 1),
		bindings:   make(chan []*networkv1.NetworkPeerBinding, 1),
		brokerConn: vnicPeer.Channel(vnic.NetworkBrokerPort, qosc),
	}
	s.negotiateNetworks = timeutil.DefaultTickEmitter.Debounce(s.runNegotiateNetworks, negotiateNetworksDebounceWait)
	return s
}

// Peer ...
type peerService struct {
	id           uint64
	vnicPeer     *vnic.Peer
	client       *networkv1.NetworkPeerClient
	caClient     *networkv1ca.CAPeerClient
	logger       *zap.Logger
	observers    *event.Observers
	broker       Broker
	vpn          *vpn.Host
	certificates *certificateMap

	negotiateNetworks timeutil.DebouncedFunc

	lock        sync.Mutex
	links       llrb.LLRB
	negotiating atomic.Bool
	keyCount    chan uint32
	bindings    chan []*networkv1.NetworkPeerBinding
	brokerConn  *vnic.FrameReadWriter
}

func (p *peerService) Negotiate(ctx context.Context, req *networkv1.NetworkPeerNegotiateRequest) (*networkv1.NetworkPeerNegotiateResponse, error) {
	select {
	case p.keyCount <- req.KeyCount:
	default:
	}

	if !p.negotiating.Load() {
		go p.negotiateNetworks(context.Background())
	}
	return &networkv1.NetworkPeerNegotiateResponse{}, nil
}

func (p *peerService) Open(ctx context.Context, req *networkv1.NetworkPeerOpenRequest) (*networkv1.NetworkPeerOpenResponse, error) {
	select {
	case p.bindings <- req.Bindings:
	default:
	}
	return &networkv1.NetworkPeerOpenResponse{}, nil
}

func (p *peerService) Close(ctx context.Context, req *networkv1.NetworkPeerCloseRequest) (*networkv1.NetworkPeerCloseResponse, error) {
	p.closeNetworkWithoutNotifyingPeer(req.Key)
	return &networkv1.NetworkPeerCloseResponse{}, nil
}

func (p *peerService) UpdateCertificate(ctx context.Context, req *networkv1.NetworkPeerUpdateCertificateRequest) (*networkv1.NetworkPeerUpdateCertificateResponse, error) {
	p.lock.Lock()
	defer p.lock.Unlock()

	if err := dao.VerifyCertificate(req.Certificate); err != nil {
		return nil, err
	}
	if !isPeerCertificateOwner(p.vnicPeer, req.Certificate) {
		return nil, ErrCertificateOwnerMismatch
	}
	if !isCertificateTrusted(req.Certificate) {
		return nil, ErrProvisionalCertificate
	}

	li := p.links.Get(&networkBinding{networkKey: dao.CertificateNetworkKey(req.Certificate)})
	if li == nil {
		return nil, ErrNetworkBindingNotFound
	}

	link := li.(*networkBinding)
	link.peerCertTrusted = true

	if err := p.openNetwork(link); err != nil {
		return nil, err
	}
	return &networkv1.NetworkPeerUpdateCertificateResponse{}, nil
}

func (p *peerService) sendCertificateUpdate(network *networkv1.Network) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	li := p.links.Get(&networkBinding{networkKey: dao.NetworkKey(network)})
	if li == nil {
		return ErrNetworkBindingNotFound
	}

	link := li.(*networkBinding)
	link.localCertTrusted = true

	err := p.client.UpdateCertificate(
		context.Background(),
		&networkv1.NetworkPeerUpdateCertificateRequest{Certificate: network.Certificate},
		&networkv1.NetworkPeerUpdateCertificateResponse{},
	)
	if err != nil {
		return err
	}

	return p.openNetwork(link)
}

func (p *peerService) closeNetworkWithoutNotifyingPeer(networkKey []byte) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	li := p.links.Get(&networkBinding{networkKey: networkKey})
	if li == nil {
		return ErrNetworkBindingNotFound
	}
	p.links.Delete(li)

	if li.(*networkBinding).open {
		node, ok := p.vpn.Node(networkKey)
		if !ok {
			return ErrNetworkNotFound
		}
		node.Network.RemovePeer(p.vnicPeer.HostID())

		defer p.observers.EmitLocal(event.NetworkPeerClose{
			PeerID:     p.id,
			NetworkID:  li.(*networkBinding).networkID,
			NetworkKey: networkKey,
		})

		p.logger.Info(
			"removed peer from network",
			zap.Stringer("peer", p.vnicPeer.HostID()),
			logutil.ByteHex("network", networkKey),
		)
	}

	if p.links.Len() == 0 {
		p.vnicPeer.Close()
	}

	return nil
}

func (p *peerService) closeNetwork(networkKey []byte) {
	p.closeNetworkWithoutNotifyingPeer(networkKey)
	p.client.Close(context.Background(), &networkv1.NetworkPeerCloseRequest{Key: networkKey}, &networkv1.NetworkPeerCloseResponse{})
}

func (p *peerService) networkKeysForLinks() [][]byte {
	p.lock.Lock()
	defer p.lock.Unlock()

	keys := make([][]byte, p.links.Len())
	p.links.AscendLessThan(llrb.Inf(1), func(li llrb.Item) bool {
		keys = append(keys, li.(*networkBinding).networkKey)
		return true
	})
	return keys
}

func (p *peerService) close() {
	for _, key := range p.networkKeysForLinks() {
		p.closeNetworkWithoutNotifyingPeer(key)
	}
}

func (p *peerService) hasNetworkBinding(networkKey []byte) bool {
	p.lock.Lock()
	defer p.lock.Unlock()
	return p.links.Has(&networkBinding{networkKey: networkKey})
}

func (p *peerService) runNegotiateNetworks(ctx context.Context) {
	if err := p.doNegotiateNetworks(ctx); err != nil {
		p.logger.Warn("network negotiation failed", zap.Error(err))
	}
}

func (p *peerService) doNegotiateNetworks(ctx context.Context) error {
	if !p.negotiating.CompareAndSwap(false, true) {
		return errors.New("cannot begin new negotiation until previous negotiation finishes")
	}
	defer p.negotiating.Store(false)

	select {
	case <-p.certificates.Loaded():
	case <-ctx.Done():
		return ctx.Err()
	}

	keys := p.certificates.Keys()
	err := p.client.Negotiate(ctx, &networkv1.NetworkPeerNegotiateRequest{KeyCount: uint32(len(keys))}, &networkv1.NetworkPeerNegotiateResponse{})
	if err != nil {
		return fmt.Errorf("sending network negotiation init failed: %w", err)
	}

	return p.exchangeBindings(ctx, keys)
}

func (p *peerService) exchangeBindings(ctx context.Context, keys [][]byte) error {
	select {
	case <-ctx.Done():
		return fmt.Errorf("no network negotiation init received: %w", ctx.Err())
	case keyCount := <-p.keyCount:
		if len(keys) == 0 || keyCount == 0 {
			return errors.New("one or both peers have zero keys")
		}

		// the psz sender role scales better than the receiver so by default we
		// pick role by comparing key counts. the role choice has to be symmetric
		// so host ids break ties.
		preferSend := p.vnicPeer.HostID().Less(p.vpn.VNIC().ID())
		if len(keys) > int(keyCount) || (len(keys) == int(keyCount) && preferSend) {
			return p.exchangeBindingsAsSender(ctx, keys)
		}
		return p.exchangeBindingsAsReceiver(ctx, keys)
	}
}

func (p *peerService) exchangeBindingsAsReceiver(ctx context.Context, keys [][]byte) error {
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

		p.observers.EmitLocal(event.NetworkPeerBindings{PeerID: p.id, NetworkKeys: keys})

		return p.handleNetworkBindings(networkBindings, peerNetworkBindings)
	}
}

func (p *peerService) exchangeBindingsAsSender(ctx context.Context, keys [][]byte) error {
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

		p.observers.EmitLocal(event.NetworkPeerBindings{PeerID: p.id, NetworkKeys: keys})

		networkBindings, err := p.sendNetworkBindings(ctx, keys)
		if err != nil {
			return err
		}

		return p.handleNetworkBindings(networkBindings, peerNetworkBindings)
	}
}

func (p *peerService) sendNetworkBindings(ctx context.Context, keys [][]byte) ([]*networkv1.NetworkPeerBinding, error) {
	var bindings []*networkv1.NetworkPeerBinding

	for _, key := range keys {
		c, ok := p.certificates.Get(key)
		if !ok {
			return nil, fmt.Errorf("certificate not found: %w", ErrNetworkNotFound)
		}

		if p.links.Has(&networkBinding{networkKey: key}) {
			continue
		}

		port, err := p.vnicPeer.ReservePort()
		if err != nil {
			return nil, err
		}

		bindings = append(
			bindings,
			&networkv1.NetworkPeerBinding{
				Port:        uint32(port),
				Certificate: c.certificate,
			},
		)
	}

	err := p.client.Open(ctx, &networkv1.NetworkPeerOpenRequest{Bindings: bindings}, &networkv1.NetworkPeerOpenResponse{})
	if err != nil {
		return nil, err
	}
	return bindings, nil
}

func (p *peerService) verifyNetworkBindings(bindings []*networkv1.NetworkPeerBinding) ([][]byte, error) {
	if bindings == nil {
		return nil, ErrNetworkBindingsEmpty
	}

	keys := make([][]byte, len(bindings))
	for i, b := range bindings {
		if err, ok := dao.VerifyCertificate(b.Certificate).(dao.Errors); ok && !err.IncludesOnly(dao.ErrNotAfterRange) {
			return nil, err
		}
		keys[i] = dao.CertificateNetworkKey(b.Certificate)
	}
	return keys, nil
}

func (p *peerService) handleNetworkBindings(networkBindings, peerNetworkBindings []*networkv1.NetworkPeerBinding) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	for i, peerBinding := range peerNetworkBindings {
		binding := networkBindings[i]
		networkKey := dao.CertificateNetworkKey(peerBinding.Certificate)

		if !isPeerCertificateOwner(p.vnicPeer, peerBinding.Certificate) {
			return ErrCertificateOwnerMismatch
		}
		if !bytes.Equal(dao.CertificateNetworkKey(binding.Certificate), networkKey) {
			return ErrNetworkAuthorityMismatch
		}
		if peerBinding.Port > uint32(math.MaxUint16) {
			return ErrNetworkPortBounds
		}

		c, ok := p.certificates.Get(networkKey)
		if !ok {
			continue
		}

		link := &networkBinding{
			networkKey:       networkKey,
			networkID:        c.networkID,
			localPort:        uint16(binding.Port),
			peerPort:         uint16(peerBinding.Port),
			localCertTrusted: isCertificateTrusted(binding.Certificate),
			peerCertTrusted:  isCertificateTrusted(peerBinding.Certificate),
		}
		p.links.ReplaceOrInsert(link)

		if !isCertificateTrusted(binding.Certificate) || !isCertificateTrusted(peerBinding.Certificate) {
			continue
		}

		if err := p.openNetwork(link); err != nil {
			return err
		}
	}
	return nil
}

func (p *peerService) openNetwork(link *networkBinding) error {
	if link.open || !link.localCertTrusted || !link.peerCertTrusted {
		return nil
	}
	link.open = true

	node, ok := p.vpn.Node(link.networkKey)
	if !ok {
		return ErrNetworkNotFound
	}
	node.Network.AddPeer(p.vnicPeer, link.localPort, link.peerPort)

	p.observers.EmitLocal(event.NetworkPeerOpen{
		PeerID:     p.id,
		NetworkID:  link.networkID,
		NetworkKey: link.networkKey,
	})

	p.logger.Info(
		"added peer to network",
		zap.Stringer("peer", p.vnicPeer.HostID()),
		logutil.ByteHex("network", link.networkKey),
		zap.Uint16("localPort", link.localPort),
		zap.Uint16("peerPort", link.peerPort),
	)

	return nil
}

type networkBinding struct {
	networkKey       []byte
	networkID        uint64
	localPort        uint16
	peerPort         uint16
	localCertTrusted bool
	peerCertTrusted  bool
	open             bool
}

func (b *networkBinding) Less(o llrb.Item) bool {
	if o, ok := o.(*networkBinding); ok {
		return bytes.Compare(b.networkKey, o.networkKey) == -1
	}
	return !o.Less(b)
}
