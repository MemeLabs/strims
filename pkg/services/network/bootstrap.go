package network

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"path"
	"runtime"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrDuplicateNetworkKey      = errors.New("duplicate network key")
	ErrNetworkNotFound          = errors.New("network not found")
	ErrPeerClosed               = errors.New("peer closed")
	ErrNetworkBindingsEmpty     = errors.New("network bindings empty")
	ErrDiscriminatorBounds      = errors.New("discriminator out of range")
	ErrNetworkOwnerMismatch     = errors.New("init and network certificate key mismatch")
	ErrNetworkAuthorityMismatch = errors.New("network ca mismatch")
	ErrNetworkIDBounds          = errors.New("network id out of range")
)

// BrokerFactory constructs network brokers from peer i/o channels
type BrokerFactory interface {
	Broker(c ReadWriteFlusher) (Broker, error)
}

// Broker negotiates shared networks between peers and binds the peers
// to the negotiated networks.
type Broker interface {
	Init(preferSender bool, keys [][]byte) error
	InitRequired() <-chan struct{}
	Keys() <-chan [][]byte
	Close()
}

// ReadWriteFlusher ...
type ReadWriteFlusher interface {
	io.ReadWriter
	Flush() error
}

// NewPeerHandler ...
func NewPeerHandler(logger *zap.Logger, factory BrokerFactory, host *vpn.Host) *PeerHandler {
	h := &PeerHandler{
		logger:  logger,
		factory: factory,
		host:    host,
	}

	host.VNIC().AddPeerHandler(h.handlePeer)

	return h
}

// PeerHandler ...
type PeerHandler struct {
	logger  *zap.Logger
	factory BrokerFactory
	host    *vpn.Host
}

func (h *PeerHandler) handlePeer(p *vnic.Peer) {
	newBootstrap(h.logger, h.factory, h.host, p)
}

func newBootstrap(logger *zap.Logger, factory BrokerFactory, host *vpn.Host, peer *vnic.Peer) *bootstrap {
	b := &bootstrap{
		logger:   logger,
		factory:  factory,
		host:     host,
		peer:     peer,
		links:    make(map[*vpn.Network]struct{}),
		bindings: make(chan *pb.NetworkHandshake_NetworkBindings),
	}

	ch := vnic.NewFrameReadWriter(peer.Link, vnic.NetworkInitPort, peer.Link.MTU())
	peer.SetHandler(vnic.NetworkInitPort, ch.HandleFrame)

	go func() {
		if err := b.readHandshakes(ch); err != nil {
			logger.Error("failed to read handshake", zap.Error(err))
		}
		peer.Close()
	}()

	bch := vnic.NewFrameReadWriter(peer.Link, vnic.NetworkBrokerPort, peer.Link.MTU())
	peer.SetHandler(vnic.NetworkBrokerPort, bch.HandleFrame)

	go func() {
		if err := b.negotiateNetworks(ch, bch); err != nil {
			logger.Error("failed to bootstrap peer networks", zap.Error(err))
		}

		peer.RemoveHandler(vnic.NetworkInitPort)
		peer.RemoveHandler(vnic.NetworkBrokerPort)

		b.removeNetworkLinks()
	}()

	return b
}

type bootstrap struct {
	logger   *zap.Logger
	factory  BrokerFactory
	host     *vpn.Host
	peer     *vnic.Peer
	links    map[*vpn.Network]struct{}
	bindings chan *pb.NetworkHandshake_NetworkBindings
}

func (h *bootstrap) negotiateNetworks(ch, bch *vnic.FrameReadWriter) (err error) {
	networks := make(chan *vpn.Network, 1)
	h.host.NotifyNetwork(networks)
	defer h.host.StopNotifyingNetwork(networks)

	broker, err := h.factory.Broker(bch)
	if err != nil {
		return err
	}
	defer broker.Close()

	if err := h.initBroker(broker); err != nil {
		return err
	}

	for {
		select {
		case keys := <-broker.Keys():
			err = h.exchangeBindingsAsSender(ch, keys)
		case handshake := <-h.bindings:
			err = h.exchangeBindingsAsReceiver(ch, handshake)
		case <-networks:
			err = h.initBroker(broker)
		case <-broker.InitRequired():
			err = h.initBroker(broker)
		case <-h.peer.Done():
			err = ErrPeerClosed
		}
		if err != nil {
			return err
		}
	}
}

func (h *bootstrap) removeNetworkLinks() {
	for n, _ := range h.links {
		h.logger.Debug(
			"removing peer from network",
			zap.Stringer("peer", h.peer.HostID()),
			logutil.ByteHex("network", n.Key()),
		)

		n.RemovePeer(h.peer.HostID())
	}
}

func (h *bootstrap) readHandshakes(ch *vnic.FrameReadWriter) error {
	for {
		var handshake pb.NetworkHandshake
		if err := protoutil.ReadStream(ch, &handshake); err != nil {
			return err
		}

		switch body := handshake.Body.(type) {
		case *pb.NetworkHandshake_NetworkBindings_:
			h.bindings <- body.NetworkBindings
		case *pb.NetworkHandshake_CertificateUpgradeOffer_:
		case *pb.NetworkHandshake_CertificateUpgradeRequest_:
		case *pb.NetworkHandshake_CertificateUpgradeResponse_:
		}
	}
}

func (h *bootstrap) initBroker(b Broker) error {
	// peers have to agree to sender/receiver preference prior to negotiation.
	// comparing host ids is arbitrary but gauranteed to be asymmetric.
	return b.Init(h.peer.HostID().Less(h.host.VNIC().ID()), h.host.NetworkKeys())
}

func (h *bootstrap) exchangeBindingsAsSender(ch *vnic.FrameReadWriter, keys [][]byte) error {
	networkBindings, err := h.sendNetworkBindings(ch, keys)
	if err != nil {
		return err
	}
	peerNetworkBindings := <-h.bindings
	if _, err = h.verifyNetworkBindings(peerNetworkBindings); err != nil {
		return err
	}
	return h.handleNetworkBindings(networkBindings, peerNetworkBindings.NetworkBindings)
}

func (h *bootstrap) exchangeBindingsAsReceiver(ch *vnic.FrameReadWriter, peerNetworkBindings *pb.NetworkHandshake_NetworkBindings) error {
	keys, err := h.verifyNetworkBindings(peerNetworkBindings)
	if err != nil {
		return err
	}
	networkBindings, err := h.sendNetworkBindings(ch, keys)
	if err != nil {
		return err
	}
	return h.handleNetworkBindings(networkBindings, peerNetworkBindings.NetworkBindings)
}

func (h *bootstrap) sendNetworkBindings(ch *vnic.FrameReadWriter, keys [][]byte) ([]*pb.NetworkHandshake_NetworkBinding, error) {
	var bindings []*pb.NetworkHandshake_NetworkBinding

	for _, key := range keys {
		c, ok := h.host.Client(key)
		if !ok {
			return nil, ErrNetworkNotFound
		}
		if _, ok := h.links[c.Network]; ok {
			continue
		}

		port, err := h.peer.ReservePort()
		if err != nil {
			return nil, err
		}

		bindings = append(
			bindings,
			&pb.NetworkHandshake_NetworkBinding{
				Port:        uint32(port),
				Certificate: c.Network.Certificate(),
			},
		)
	}
	err := protoutil.WriteStream(ch, &pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_NetworkBindings_{
			NetworkBindings: &pb.NetworkHandshake_NetworkBindings{
				NetworkBindings: bindings,
			},
		},
	})
	if err != nil {
		return nil, err
	}
	if err := ch.Flush(); err != nil {
		return nil, err
	}
	return bindings, nil
}

func (h *bootstrap) verifyNetworkBindings(bindings *pb.NetworkHandshake_NetworkBindings) ([][]byte, error) {
	if bindings == nil {
		return nil, ErrNetworkBindingsEmpty
	}

	keys := make([][]byte, len(bindings.NetworkBindings))
	for i, b := range bindings.NetworkBindings {
		if err := dao.VerifyCertificate(b.Certificate); err != nil {
			return nil, err
		}
		keys[i] = dao.GetRootCert(b.Certificate).Key
	}
	return keys, nil
}

func (h *bootstrap) handleNetworkBindings(networkBindings, peerNetworkBindings []*pb.NetworkHandshake_NetworkBinding) error {
	for i, pb := range peerNetworkBindings {
		b := networkBindings[i]

		if !bytes.Equal(h.peer.Certificate.Key, pb.Certificate.Key) {
			return ErrNetworkOwnerMismatch
		}
		if !bytes.Equal(dao.GetRootCert(b.Certificate).Key, dao.GetRootCert(pb.Certificate).Key) {
			return ErrNetworkAuthorityMismatch
		}
		if pb.Port > uint32(math.MaxUint16) {
			return ErrNetworkIDBounds
		}

		if !bytes.Equal(dao.GetRootCert(b.Certificate).Key, pb.Certificate.GetParent().Key) {
			jsonDump(pb)
		}

		// TODO: if the peer has a provisional certificate offer mediation

		c, ok := h.host.Client(dao.GetRootCert(pb.Certificate).Key)
		if !ok {
			return ErrNetworkNotFound
		}

		h.logger.Debug(
			"adding peer to network",
			zap.Stringer("peer", h.peer.HostID()),
			logutil.ByteHex("network", dao.GetRootCert(pb.Certificate).Key),
			zap.Uint32("localPort", b.Port),
			zap.Uint32("remotePort", pb.Port),
		)

		c.Network.AddPeer(h.peer, uint16(b.Port), uint16(pb.Port))
		h.links[c.Network] = struct{}{}

		// h.host.peerNetworkObservers.Emit(PeerNetwork{h.peer, c.Network})
	}
	return nil
}

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}
