package vpn

import (
	"bytes"
	"math"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"go.uber.org/zap"
)

func newBootstrap(logger *zap.Logger, n *Host, peer *vnic.Peer) *bootstrap {
	b := &bootstrap{
		logger:     logger,
		host:       n,
		peer:       peer,
		links:      make(map[*Network]*networkLink),
		handshakes: make(chan *pb.NetworkHandshake),
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
	logger     *zap.Logger
	host       *Host
	peer       *vnic.Peer
	links      map[*Network]*networkLink
	handshakes chan *pb.NetworkHandshake
}

func (h *bootstrap) negotiateNetworks(ch, bch *vnic.FrameReadWriter) (err error) {
	networks := make(chan *Network, 1)
	h.host.NotifyNetwork(networks)
	defer h.host.StopNotifyingNetwork(networks)

	broker, err := h.host.brokerFactory.Broker(bch)
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
		case handshake := <-h.handshakes:
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
	for n, l := range h.links {
		h.logger.Debug(
			"removing peer from network",
			zap.Stringer("peer", l.hostID),
			logutil.ByteHex("network", dao.GetRootCert(n.certificate).Key),
		)

		n.removeLink(l)
	}
}

func (h *bootstrap) readHandshakes(ch *vnic.FrameReadWriter) error {
	for {
		var handshake pb.NetworkHandshake
		if err := protoutil.ReadStream(ch, &handshake); err != nil {
			return err
		}
		h.handshakes <- &handshake
	}
}

func (h *bootstrap) initBroker(b Broker) error {
	// peers have to agree to sender/receiver preference prior to negotiation.
	// comparing host ids is arbitrary but gauranteed to be asymmetric.
	return b.Init(h.peer.HostID().Less(h.host.ID()), h.host.NetworkKeys())
}

func (h *bootstrap) exchangeBindingsAsSender(ch *vnic.FrameReadWriter, keys [][]byte) error {
	networkBindings, err := h.sendNetworkBindings(ch, keys)
	if err != nil {
		return err
	}
	handshake := <-h.handshakes
	peerNetworkBindings := handshake.GetNetworkBindings()
	if _, err = h.verifyNetworkBindings(peerNetworkBindings); err != nil {
		return err
	}
	return h.handleNetworkBindings(networkBindings, peerNetworkBindings.NetworkBindings)
}

func (h *bootstrap) exchangeBindingsAsReceiver(ch *vnic.FrameReadWriter, handshake *pb.NetworkHandshake) error {
	peerNetworkBindings := handshake.GetNetworkBindings()
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
				Certificate: c.Network.certificate,
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

		link := &networkLink{
			hostID:          h.peer.HostID(),
			FrameReadWriter: vnic.NewFrameReadWriter(h.peer.Link, uint16(pb.Port), h.peer.Link.MTU()),
		}
		h.links[c.Network] = link
		c.Network.addLink(link)

		h.peer.SetHandler(uint16(b.Port), c.Network.handleFrame)

		h.host.peerNetworkObservers.Emit(PeerNetwork{h.peer, c.Network})
	}
	return nil
}
