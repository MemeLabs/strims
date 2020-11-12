package network

import (
	"bytes"
	"context"
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

// Broker negotiates common networks with peers.
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
func NewPeerHandler(logger *zap.Logger, broker Broker, host *vpn.Host, profile *pb.Profile) *PeerHandler {
	h := &PeerHandler{
		logger:  logger,
		broker:  broker,
		host:    host,
		profile: profile,
	}

	host.VNIC().AddPeerHandler(h.handlePeer)

	return h
}

// PeerHandler ...
type PeerHandler struct {
	logger  *zap.Logger
	broker  Broker
	host    *vpn.Host
	profile *pb.Profile
}

func (h *PeerHandler) handlePeer(p *vnic.Peer) {
	newBootstrap(h.logger, h.broker, h.host, h.profile, p)
}

func newBootstrap(logger *zap.Logger, broker Broker, host *vpn.Host, profile *pb.Profile, peer *vnic.Peer) *bootstrap {
	h := &bootstrap{
		logger:     logger,
		broker:     broker,
		host:       host,
		profile:    profile,
		peer:       peer,
		links:      make(map[*vpn.Network]struct{}),
		peerInit:   make(chan *pb.NetworkHandshake_Init, 1),
		bindings:   make(chan *pb.NetworkHandshake_NetworkBindings),
		conn:       peer.Channel(vnic.NetworkInitPort),
		brokerConn: peer.Channel(vnic.NetworkBrokerPort),
	}

	go h.run()

	return h
}

type bootstrap struct {
	logger     *zap.Logger
	broker     Broker
	host       *vpn.Host
	peer       *vnic.Peer
	profile    *pb.Profile
	links      map[*vpn.Network]struct{}
	syncing    int32
	peerInit   chan *pb.NetworkHandshake_Init
	bindings   chan *pb.NetworkHandshake_NetworkBindings
	conn       *vnic.FrameReadWriter
	brokerConn *vnic.FrameReadWriter
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
		if err := protoutil.ReadStream(h.conn, &handshake); err != nil {
			return err
		}

		switch body := handshake.Body.(type) {
		case *pb.NetworkHandshake_Init_:
			h.peerInit <- body.Init
			h.sync()
		case *pb.NetworkHandshake_NetworkBindings_:
			h.bindings <- body.NetworkBindings
		case *pb.NetworkHandshake_CertificateUpgradeOffer_:
			h.handleCertificateUpgradeOffer(body.CertificateUpgradeOffer)
		case *pb.NetworkHandshake_CertificateUpgradeRequest_:
			h.handleCertificateUpgradeRequest(body.CertificateUpgradeRequest)
		case *pb.NetworkHandshake_CertificateUpgradeResponse_:
			h.handleCertificateUpgradeResponse(body.CertificateUpgradeResponse)
		case *pb.NetworkHandshake_CertificateUpdate_:
			h.handleCertificateUpdate(body.CertificateUpdate)
		}
	}
}

func (h *bootstrap) handleCertificateUpgradeOffer(req *pb.NetworkHandshake_CertificateUpgradeOffer) error {
	h.logger.Debug("handleCertificateUpgradeOffer")
	jsonDump(req)
	// TODO: deduplicate requests

	csr, err := dao.NewCertificateRequest(
		h.profile.Key,
		pb.KeyUsage_KEY_USAGE_PEER|pb.KeyUsage_KEY_USAGE_SIGN,
		dao.WithSubject(h.profile.Name),
	)
	if err != nil {
		return err
	}

	client, ok := h.host.Client(req.NetworkKey)
	if !ok {
		return errors.New("network not found")
	}

	// TODO: do we need to upgrade this cert? (invite/expired)
	// TODO: are we already upgrading this cert via some other peer?

	return h.send(&pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_CertificateUpgradeRequest_{
			CertificateUpgradeRequest: &pb.NetworkHandshake_CertificateUpgradeRequest{
				Certificate:        client.Network.Certificate(),
				CertificateRequest: csr,
			},
		},
	})
}

func (h *bootstrap) handleCertificateUpgradeRequest(req *pb.NetworkHandshake_CertificateUpgradeRequest) error {
	h.logger.Debug("handleCertificateUpgradeRequest")
	jsonDump(req)

	networkKey := dao.GetRootCert(req.Certificate).Key

	renewReq := &pb.CARenewRequest{
		Certificate:        req.Certificate,
		CertificateRequest: req.CertificateRequest,
	}
	renewRes := &pb.CARenewResponse{}
	err := func() error {
		client, ok := h.host.Client(networkKey)
		if !ok {
			return errors.New("network not found")
		}

		caClient, err := NewCAClient(h.logger, client)
		if err != nil {
			return err
		}

		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		return caClient.Renew(ctx, renewReq, renewRes)
	}()

	res := &pb.NetworkHandshake_CertificateUpgradeResponse{
		NetworkKey: networkKey,
	}
	if err != nil {
		res.Body = &pb.NetworkHandshake_CertificateUpgradeResponse_Error{
			Error: err.Error(),
		}
	} else {
		res.Body = &pb.NetworkHandshake_CertificateUpgradeResponse_Certificate{
			Certificate: renewRes.Certificate,
		}
	}

	return h.send(&pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_CertificateUpgradeResponse_{
			CertificateUpgradeResponse: res,
		},
	})
}

func (h *bootstrap) handleCertificateUpgradeResponse(req *pb.NetworkHandshake_CertificateUpgradeResponse) error {
	h.logger.Debug("handleCertificateUpgradeResponse")
	jsonDump(req)

	if err := req.GetError(); err != "" {
		// TODO: propagate to user
		return errors.New(err)
	}

	// TODO: send to all pending peers
	// TODO: validate cert
	// TODO: confirm that we want to replace the cert

	return h.send(&pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_CertificateUpdate_{
			CertificateUpdate: &pb.NetworkHandshake_CertificateUpdate{
				Certificate: req.GetCertificate(),
			},
		},
	})
}

func (h *bootstrap) handleCertificateUpdate(req *pb.NetworkHandshake_CertificateUpdate) {
	h.logger.Debug("handleCertificateUpdate")
	jsonDump(req)

	// TODO: finish binding
}

func (h *bootstrap) send(msg protoreflect.ProtoMessage) error {
	err := protoutil.WriteStream(h.conn, msg)
	if err != nil {
		return err
	}
	return h.conn.Flush()
}

func (h *bootstrap) sync() {
	if !atomic.CompareAndSwapInt32(&h.syncing, 0, 1) {
		return
	}

	keys := h.host.NetworkKeys()
	err := h.send(&pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_Init_{
			Init: &pb.NetworkHandshake_Init{
				KeyCount: int32(len(keys)),
			},
		},
	})
	if err != nil {
		atomic.StoreInt32(&h.syncing, 0)
		h.logger.Debug("sync failed", zap.Error(err))
		return
	}

	go func() {
		if err := h.exchangeBindings(keys); err != nil {
			h.logger.Debug("sync failed", zap.Error(err))
		}
		atomic.StoreInt32(&h.syncing, 0)
	}()
}

func (h *bootstrap) exchangeBindings(keys [][]byte) error {
	select {
	case <-time.After(time.Second * 10):
		return errors.New("timeout")
	case peerInit := <-h.peerInit:
		if len(keys) == 0 || peerInit.KeyCount == 0 {
			return nil
		}

		preferSend := h.peer.HostID().Less(h.host.VNIC().ID())
		if len(keys) > int(peerInit.KeyCount) || (len(keys) == int(peerInit.KeyCount) && preferSend) {
			return h.exchangeBindingsAsSender(keys)
		} else {
			return h.exchangeBindingsAsReceiver(keys)
		}
	}
}

func (h *bootstrap) exchangeBindingsAsReceiver(keys [][]byte) error {
	keys, err := h.broker.ReceiveKeys(h.brokerConn, keys)
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

func (h *bootstrap) exchangeBindingsAsSender(keys [][]byte) error {
	if err := h.broker.SendKeys(h.brokerConn, keys); err != nil {
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
	err := protoutil.WriteStream(h.conn, &pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_NetworkBindings_{
			NetworkBindings: &pb.NetworkHandshake_NetworkBindings{
				NetworkBindings: bindings,
			},
		},
	})
	if err != nil {
		return nil, err
	}
	if err := h.conn.Flush(); err != nil {
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
	for i, peerBinding := range peerNetworkBindings {
		binding := networkBindings[i]

		if !bytes.Equal(h.peer.Certificate.Key, peerBinding.Certificate.Key) {
			return ErrNetworkOwnerMismatch
		}
		if !bytes.Equal(dao.GetRootCert(binding.Certificate).Key, dao.GetRootCert(peerBinding.Certificate).Key) {
			return ErrNetworkAuthorityMismatch
		}
		if peerBinding.Port > uint32(math.MaxUint16) {
			return ErrNetworkIDBounds
		}

		// handle invite certs
		// TODO: also handle expired certificates
		if !bytes.Equal(dao.GetRootCert(binding.Certificate).Key, peerBinding.Certificate.GetParent().Key) {
			// jsonDump(peerBinding)
			err := h.send(&pb.NetworkHandshake{
				Body: &pb.NetworkHandshake_CertificateUpgradeOffer_{
					CertificateUpgradeOffer: &pb.NetworkHandshake_CertificateUpgradeOffer{
						NetworkKey: dao.GetRootCert(binding.Certificate).Key,
					},
				},
			})
			if err != nil {
				h.logger.Debug("cert upgrade offer failed", zap.Error(err))
			}
		}

		c, ok := h.host.Client(dao.GetRootCert(peerBinding.Certificate).Key)
		if !ok {
			return ErrNetworkNotFound
		}

		h.logger.Debug(
			"adding peer to network",
			zap.Stringer("peer", h.peer.HostID()),
			logutil.ByteHex("network", dao.GetRootCert(peerBinding.Certificate).Key),
			zap.Uint32("localPort", binding.Port),
			zap.Uint32("remotePort", peerBinding.Port),
		)

		c.Network.AddPeer(h.peer, uint16(binding.Port), uint16(peerBinding.Port))
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
