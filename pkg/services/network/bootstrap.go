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
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/reflect/protoreflect"
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

type Broker interface {
	SendKeys(c ReadWriteFlusher, keys [][]byte) error
	ReceiveKeys(c ReadWriteFlusher, keys [][]byte) ([][]byte, error)
}

// ReadWriteFlusher ...
type ReadWriteFlusher interface {
	io.ReadWriter
	Flush() error
}

// NewPeerHandler ...
func NewPeerHandler(logger *zap.Logger, broker Broker, host *vpn.Host) *PeerHandler {
	h := &PeerHandler{
		logger: logger,
		broker: broker,
		host:   host,
	}

	host.VNIC().AddPeerHandler(h.handlePeer)

	return h
}

// PeerHandler ...
type PeerHandler struct {
	logger *zap.Logger
	broker Broker
	host   *vpn.Host
}

func (h *PeerHandler) handlePeer(p *vnic.Peer) {
	newBootstrap(h.logger, h.broker, h.host, p)
}

func newBootstrap(logger *zap.Logger, broker Broker, host *vpn.Host, peer *vnic.Peer) *bootstrap {
	h := &bootstrap{
		logger:   logger,
		broker:   broker,
		host:     host,
		peer:     peer,
		links:    make(map[*vpn.Network]struct{}),
		peerInit: make(chan *pb.NetworkHandshake_Init, 1),
		bindings: make(chan *pb.NetworkHandshake_NetworkBindings),
		ch:       peer.Channel(vnic.NetworkInitPort),
	}

	go h.run()

	return h
}

type bootstrap struct {
	logger   *zap.Logger
	broker   Broker
	host     *vpn.Host
	peer     *vnic.Peer
	links    map[*vpn.Network]struct{}
	syncing  int32
	peerInit chan *pb.NetworkHandshake_Init
	bindings chan *pb.NetworkHandshake_NetworkBindings
	ch       *vnic.FrameReadWriter
}

func (h *bootstrap) run() {
	networks := make(chan *vpn.Network, 1)
	h.host.NotifyNetwork(networks)

	go func() {
		h.sync()
		for range networks {
			h.sync()
		}
	}()

	if err := h.readHandshakes(); err != nil {
		h.logger.Error("failed to read handshake", zap.Error(err))
	}

	h.host.StopNotifyingNetwork(networks)
	close(networks)

	h.removeNetworkLinks()
	h.peer.Close()
}

func (h *bootstrap) removeNetworkLinks() {
	for n := range h.links {
		h.logger.Debug(
			"removing peer from network",
			zap.Stringer("peer", h.peer.HostID()),
			logutil.ByteHex("network", n.Key()),
		)

		n.RemovePeer(h.peer.HostID())
	}
}

func (h *bootstrap) readHandshakes() error {
	for {
		var handshake pb.NetworkHandshake
		if err := protoutil.ReadStream(h.ch, &handshake); err != nil {
			return err
		}

		switch body := handshake.Body.(type) {
		case *pb.NetworkHandshake_Init_:
			h.peerInit <- body.Init
			h.sync()
		case *pb.NetworkHandshake_NetworkBindings_:
			h.bindings <- body.NetworkBindings
		case *pb.NetworkHandshake_CertificateUpgradeOffer_:
		case *pb.NetworkHandshake_CertificateUpgradeRequest_:
		case *pb.NetworkHandshake_CertificateUpgradeResponse_:
		}
	}
}

func (h *bootstrap) send(msg protoreflect.ProtoMessage) error {
	err := protoutil.WriteStream(h.ch, msg)
	if err != nil {
		return err
	}
	return h.ch.Flush()
}

func (h *bootstrap) sync() {
	if !atomic.CompareAndSwapInt32(&h.syncing, 0, 1) {
		return
	}

	keys := h.host.NetworkKeys()
	ch := h.peer.Channel(vnic.NetworkBrokerPort)

	err := h.send(&pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_Init_{
			Init: &pb.NetworkHandshake_Init{
				KeyCount: int32(len(keys)),
			},
		},
	})
	if err != nil {
		atomic.StoreInt32(&h.syncing, 0)
		h.logger.Debug("sync failedd", zap.Error(err))
		return
	}

	go func() {
		if err := h.exchangeBindings(ch, keys); err != nil {
			h.logger.Debug("sync failedd", zap.Error(err))
		}
		h.peer.CloseChannel(ch)
		atomic.StoreInt32(&h.syncing, 0)
	}()
}

func (h *bootstrap) exchangeBindings(ch *vnic.FrameReadWriter, keys [][]byte) error {
	select {
	case <-time.After(time.Second * 10):
		return errors.New("timeout")
	case peerInit := <-h.peerInit:
		preferSend := h.peer.HostID().Less(h.host.VNIC().ID())
		if len(keys) > int(peerInit.KeyCount) || (len(keys) == int(peerInit.KeyCount) && preferSend) {
			return h.exchangeBindingsAsSender(ch, keys)
		} else {
			return h.exchangeBindingsAsReceiver(ch, keys)
		}
	}
}

func (h *bootstrap) exchangeBindingsAsReceiver(ch *vnic.FrameReadWriter, keys [][]byte) error {
	keys, err := h.broker.ReceiveKeys(ch, keys)
	if err != nil {
		return err
	}
	networkBindings, err := h.sendNetworkBindings(keys)
	if err != nil {
		return err
	}
	peerNetworkBindings := <-h.bindings
	if _, err = h.verifyNetworkBindings(peerNetworkBindings); err != nil {
		return err
	}
	return h.handleNetworkBindings(networkBindings, peerNetworkBindings.NetworkBindings)
}

func (h *bootstrap) exchangeBindingsAsSender(ch *vnic.FrameReadWriter, keys [][]byte) error {
	if err := h.broker.SendKeys(ch, keys); err != nil {
		return err
	}
	peerNetworkBindings := <-h.bindings
	keys, err := h.verifyNetworkBindings(peerNetworkBindings)
	if err != nil {
		return err
	}
	networkBindings, err := h.sendNetworkBindings(keys)
	if err != nil {
		return err
	}
	return h.handleNetworkBindings(networkBindings, peerNetworkBindings.NetworkBindings)
}

func (h *bootstrap) sendNetworkBindings(keys [][]byte) ([]*pb.NetworkHandshake_NetworkBinding, error) {
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
	err := protoutil.WriteStream(h.ch, &pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_NetworkBindings_{
			NetworkBindings: &pb.NetworkHandshake_NetworkBindings{
				NetworkBindings: bindings,
			},
		},
	})
	if err != nil {
		return nil, err
	}
	if err := h.ch.Flush(); err != nil {
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
