package network

import (
	"context"
	"errors"
	"math"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/services/ca"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// errors ...
var (
	ErrNetworkNotFound          = errors.New("network not found")
	ErrNetworkBindingsEmpty     = errors.New("network bindings empty")
	ErrNetworkBindingNotFound   = errors.New("network binding not found")
	ErrNetworkAuthorityMismatch = errors.New("network ca mismatch")
	ErrCertificateOwnerMismatch = errors.New("init and network certificate key mismatch")
	ErrProvisionalCertificate   = errors.New("provisional certificate is not supported")
	ErrNetworkPortBounds        = errors.New("network port out of range")
)

const certRecheckInterval = time.Minute * 5
const certRenewScheduleAheadDuration = time.Hour * 24 * 7

// Broker negotiates common networks with peers.
type Broker interface {
	SendKeys(c ioutil.ReadWriteFlusher, keys [][]byte) error
	ReceiveKeys(c ioutil.ReadWriteFlusher, keys [][]byte) ([][]byte, error)
}

// NewControl ...
func NewControl(logger *zap.Logger, broker Broker, vpn *vpn.Host, store *dao.ProfileStore, profile *pb.Profile, observers *event.Observers, dialer *dialer.Control) *Control {
	events := make(chan interface{}, 128)
	observers.Global.Notify(events)

	return &Control{
		logger:           logger,
		broker:           broker,
		vpn:              vpn,
		store:            store,
		profile:          profile,
		observers:        observers,
		events:           events,
		dialer:           dialer,
		certRenewTimeout: time.NewTimer(0),
		networks:         map[uint64]*pb.Network{},
		peers:            map[uint64]*Peer{},
		certificates:     &certificateMap{},
	}
}

// Control ...
type Control struct {
	logger  *zap.Logger
	broker  Broker
	vpn     *vpn.Host
	store   *dao.ProfileStore
	profile *pb.Profile

	lock              sync.Mutex
	observers         *event.Observers
	events            chan interface{}
	dialer            *dialer.Control
	certRenewTimeout  *time.Timer
	nextCertRenewTime time.Time
	networks          map[uint64]*pb.Network
	peers             map[uint64]*Peer
	certificates      *certificateMap
}

// Run ...
func (t *Control) Run(ctx context.Context) {
	t.startNetworks()
	t.scheduleCertRenewal()

	for {
		select {
		case <-t.certRenewTimeout.C:
			t.renewExpiredCerts()
		case e := <-t.events:
			switch e := e.(type) {
			case event.PeerAdd:
				t.handlePeerAdd(ctx, e.ID)
			case event.NetworkPeerBindings:
				t.handlePeerBinding(ctx, e.PeerID, e.NetworkKeys)
			case event.NetworkCertUpdate:
				t.handleNetworkCertUpdate(e.Network)
			case event.NetworkAdd:
				t.handleNetworkAdd(ctx)
			}
		case <-ctx.Done():
			return
		}

		t.scheduleCertRenewal()
	}
}

func (t *Control) handlePeerAdd(ctx context.Context, peerID uint64) {
	t.lock.Lock()
	defer t.lock.Unlock()

	peer, ok := t.peers[peerID]
	if !ok {
		return
	}

	go func() {
		if err := peer.negotiateNetworks(ctx); err != nil {
			t.logger.Debug("network negotiation failed", zap.Error(err))
		}
		t.observers.Local.Emit(event.NetworkNegotiationComplete{})
	}()
}

func (t *Control) handlePeerBinding(ctx context.Context, peerID uint64, networkKeys [][]byte) {
	t.lock.Lock()
	defer t.lock.Unlock()

	peer, ok := t.peers[peerID]
	if !ok {
		return
	}

	for _, key := range networkKeys {
		c, ok := t.certificates.Get(key)
		if !ok || c.trusted {
			continue
		}

		network, ok := t.networks[c.networkID]
		if !ok {
			continue
		}

		go func() {
			if err := t.renewCertificateWithPeer(ctx, network, peer); err != nil {
				t.logger.Debug(
					"certificate renew via peer failed",
					zap.Stringer("peer", peer.peer.HostID()),
					logutil.ByteHex("network", networkKeyForCertificate(network.Certificate)),
					zap.Error(err),
				)
			}
		}()
	}
}

func (t *Control) handleNetworkCertUpdate(network *pb.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	networkKey := networkKeyForCertificate(network.Certificate)
	for _, peer := range t.peers {
		if peer.hasNetworkBinding(networkKey) {
			go peer.sendCertificateUpdate(network)
		}
	}
}

func (t *Control) handleNetworkAdd(ctx context.Context) {
	t.lock.Lock()
	defer t.lock.Unlock()

	// TODO: throttle
	for _, peer := range t.peers {
		peer := peer
		go func() {
			if err := peer.negotiateNetworks(ctx); err != nil {
				t.logger.Debug("network negotiation failed", zap.Error(err))
			}
		}()
	}
}

type certificateRenewFunc func(ctx context.Context, cert *pb.Certificate, csr *pb.CertificateRequest) (*pb.Certificate, error)

// renewCertificateWithRenewFunc ...
func (t *Control) renewCertificateWithRenewFunc(network *pb.Network, fn certificateRenewFunc) error {
	subject := t.profile.Name
	if network.AltProfileName != "" {
		subject = network.AltProfileName
	}

	csr, err := dao.NewCertificateRequest(
		t.profile.Key,
		pb.KeyUsage_KEY_USAGE_PEER|pb.KeyUsage_KEY_USAGE_SIGN,
		dao.WithSubject(subject),
	)
	if err != nil {
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	cert, err := fn(ctx, network.Certificate, csr)
	if err != nil {
		return err
	}
	if err := dao.VerifyCertificate(cert); err != nil {
		return err
	}

	return t.setCertificate(network.Id, cert)
}

// renewCertificate ...
func (t *Control) renewCertificate(network *pb.Network) error {
	return t.renewCertificateWithRenewFunc(
		network,
		func(ctx context.Context, cert *pb.Certificate, csr *pb.CertificateRequest) (*pb.Certificate, error) {
			networkKey := networkKeyForCertificate(network.Certificate)
			client, err := t.dialer.Client(networkKey, networkKey, ca.AddressSalt)
			if err != nil {
				return nil, err
			}
			caClient := api.NewCAClient(client)

			renewReq := &pb.CARenewRequest{
				Certificate:        cert,
				CertificateRequest: csr,
			}
			renewRes := &pb.CARenewResponse{}
			if err := caClient.Renew(ctx, renewReq, renewRes); err != nil {
				return nil, err
			}

			return renewRes.Certificate, nil
		},
	)
}

func (t *Control) renewCertificateWithPeer(ctx context.Context, network *pb.Network, peer *Peer) error {
	return t.renewCertificateWithRenewFunc(
		network,
		func(ctx context.Context, cert *pb.Certificate, csr *pb.CertificateRequest) (*pb.Certificate, error) {
			req := &pb.CAPeerRenewRequest{
				Certificate:        cert,
				CertificateRequest: csr,
			}
			res := &pb.CAPeerRenewResponse{}
			if err := peer.client.CA().Renew(ctx, req, res); err != nil {
				return nil, err
			}
			return res.Certificate, nil
		},
	)
}

// AddPeer ...
func (t *Control) AddPeer(id uint64, peer *vnic.Peer, client PeerClient) *Peer {
	p := NewPeer(id, peer, client, t.logger, t.observers, t.broker, t.vpn, t.certificates)

	t.lock.Lock()
	defer t.lock.Unlock()

	t.peers[p.id] = p

	return p
}

// RemovePeer ...
func (t *Control) RemovePeer(id uint64) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.peers[id].close()
	delete(t.peers, id)
}

func (t *Control) startNetworks() {
	t.lock.Lock()
	defer t.lock.Unlock()

	networks, err := dao.GetNetworks(t.store)
	if err != nil {
		t.logger.Fatal("loading networks failed", zap.Error(err))
	}

	for _, network := range networks {
		t.networks[network.Id] = network
		t.certificates.Insert(network)
		t.dialer.ReplaceOrInsertNetwork(network)

		cert := dao.GetRootCert(network.Certificate)

		if _, err := t.vpn.AddNetwork(network.Certificate); err != nil {
			t.logger.Error(
				"starting network failed",
				zap.String("name", cert.Subject),
				logutil.ByteHex("key", cert.Key),
				zap.Error(err),
			)
		} else {
			t.logger.Info(
				"network started",
				zap.String("name", cert.Subject),
				logutil.ByteHex("key", cert.Key),
			)

			t.observers.Local.Emit(event.NetworkStart{Network: network})
		}
	}
}

func (t *Control) scheduleCertRenewal() {
	t.lock.Lock()
	defer t.lock.Unlock()

	minNextTime := time.Unix(math.MaxInt64, 0)

	for _, n := range t.networks {
		nextTime := nextCertificateRenewTime(n)
		if nextTime.Before(minNextTime) {
			minNextTime = nextTime
		}
	}

	now := time.Now()

	if minNextTime.Before(now) {
		minNextTime = now
	}
	if minNextTime.Before(t.nextCertRenewTime) {
		minNextTime = t.nextCertRenewTime
	}

	t.certRenewTimeout.Reset(minNextTime.Sub(now))
}

func (t *Control) renewExpiredCerts() {
	t.lock.Lock()
	defer t.lock.Unlock()

	now := time.Now()
	t.nextCertRenewTime = now.Add(certRecheckInterval)

	for _, network := range t.networks {
		network := network

		if now.After(nextCertificateRenewTime(network)) || isCertificateSubjectMismatched(network) {
			go func() {
				if err := t.renewCertificate(network); err != nil {
					t.logger.Debug(
						"network certificate renewal failed",
						logutil.ByteHex("network", networkKeyForCertificate(network.Certificate)),
						zap.Error(err),
					)
				}
			}()
		}
	}
}

func (t *Control) mutateNetworkWithFinalizer(id uint64, mutate func(*pb.Network) error, finalize func(*pb.Network)) error {
	t.lock.Lock()
	defer t.lock.Unlock()

	network, ok := t.networks[id]
	if !ok {
		return ErrNetworkNotFound
	}

	clone := proto.Clone(network).(*pb.Network)
	if err := mutate(clone); err != nil {
		return err
	}

	if err := dao.UpsertNetwork(t.store, clone); err != nil {
		return err
	}

	t.networks[id] = clone

	finalize(clone)
	return nil
}

func noopMutateNetworkFinalizer(*pb.Network) {}

func (t *Control) mutateNetwork(id uint64, mutate func(*pb.Network) error) error {
	return t.mutateNetworkWithFinalizer(id, mutate, noopMutateNetworkFinalizer)
}

func (t *Control) setCertificate(id uint64, cert *pb.Certificate) error {
	return t.mutateNetworkWithFinalizer(
		id,
		func(network *pb.Network) error {
			network.Certificate = cert
			return nil
		},
		func(network *pb.Network) {
			t.certificates.Insert(network)
			t.dialer.ReplaceOrInsertNetwork(network)
			t.observers.Global.Emit(event.NetworkCertUpdate{Network: proto.Clone(network).(*pb.Network)})
		},
	)
}

func (t *Control) setNetworkAltProfileName(id uint64, name string) error {
	return t.mutateNetwork(id, func(network *pb.Network) error {
		network.AltProfileName = name
		return nil
	})
}

// Certificate ...
func (t *Control) Certificate(networkKey []byte) (*pb.Certificate, bool) {
	if ci, ok := t.certificates.Get(networkKey); ok {
		return ci.certificate, true
	}
	return nil, false
}

// Add ...
func (t *Control) Add(network *pb.Network) error {
	t.lock.Lock()
	defer t.lock.Unlock()

	if _, ok := t.networks[network.Id]; ok {
		return errors.New("duplicate network id")
	}

	if err := dao.UpsertNetwork(t.store, network); err != nil {
		return err
	}

	if _, err := t.vpn.AddNetwork(network.Certificate); err != nil {
		return err
	}

	t.networks[network.Id] = network
	t.certificates.Insert(network)
	t.dialer.ReplaceOrInsertNetwork(network)

	t.observers.Global.Emit(event.NetworkAdd{Network: network})
	t.observers.Local.Emit(event.NetworkStart{Network: network})

	return nil
}

// Remove ...
func (t *Control) Remove(id uint64) error {
	t.lock.Lock()
	defer t.lock.Unlock()

	network, ok := t.networks[id]
	if !ok {
		return ErrNetworkNotFound
	}
	networkKey := networkKeyForCertificate(network.Certificate)

	if err := dao.DeleteNetwork(t.store, id); err != nil {
		return err
	}

	for _, p := range t.peers {
		p.closeNetwork(networkKey)
	}

	if err := t.vpn.RemoveNetwork(dao.GetRootCert(network.Certificate).Key); err != nil {
		return err
	}

	t.dialer.RemoveNetwork(network)
	t.certificates.Delete(networkKey)
	delete(t.networks, id)

	t.observers.Local.Emit(event.NetworkStop{Network: network})
	t.observers.Global.Emit(event.NetworkRemove{Network: network})

	return nil
}
