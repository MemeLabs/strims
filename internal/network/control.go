package network

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/api"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/notification"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	notificationv1 "github.com/MemeLabs/go-ppspp/pkg/apis/notification/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
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

type Control interface {
	CA() CA
	Dialer() Dialer
	Run(ctx context.Context)
	AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer
	RemovePeer(id uint64)
	Certificate(networkKey []byte) (*certificate.Certificate, bool)
	Add(n *networkv1.Network, certLogs []*networkv1ca.CertificateLog) error
	Remove(id uint64) error
	ReadEvents(ctx context.Context) <-chan *networkv1.NetworkEvent
	UpdateDisplayOrder(ids []uint64) error
}

// Broker negotiates common networks with peers.
type Broker interface {
	SendKeys(c ioutil.ReadWriteFlusher, keys [][]byte) error
	ReceiveKeys(c ioutil.ReadWriteFlusher, keys [][]byte) ([][]byte, error)
}

// NewControl ...
func NewControl(logger *zap.Logger, broker Broker, vpn *vpn.Host, store *dao.ProfileStore, profile *profilev1.Profile, observers *event.Observers, notification notification.Control) Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	dialer := newDialer(logger, vpn, profile.Key)

	return &control{
		logger:           logger,
		broker:           broker,
		vpn:              vpn,
		qosc:             vpn.VNIC().QOS().AddClass(1),
		store:            store,
		profile:          profile,
		observers:        observers,
		events:           events,
		notification:     notification,
		ca:               newCA(logger, vpn, store, observers, dialer),
		dialer:           dialer,
		certRenewTimeout: time.NewTimer(0),
		networks:         map[uint64]*network{},
		peers:            map[uint64]*peer{},
		certificates:     newCertificateMap(),
	}
}

// Control ...
type control struct {
	logger  *zap.Logger
	broker  Broker
	vpn     *vpn.Host
	qosc    *qos.Class
	store   *dao.ProfileStore
	profile *profilev1.Profile

	lock              sync.Mutex
	observers         *event.Observers
	events            chan interface{}
	notification      notification.Control
	ca                *ca
	dialer            *dialer
	certRenewTimeout  *time.Timer
	nextCertRenewTime timeutil.Time
	networks          map[uint64]*network
	peers             map[uint64]*peer
	certificates      *certificateMap
}

func (t *control) CA() CA {
	return t.ca
}

func (t *control) Dialer() Dialer {
	return t.dialer
}

// Run ...
func (t *control) Run(ctx context.Context) {
	go t.ca.Run(ctx)

	t.startNetworks()
	t.scheduleCertRenewal()

	for {
		select {
		case <-t.certRenewTimeout.C:
			t.renewExpiredCerts(ctx)
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
			case event.NetworkPeerOpen:
				t.handleNetworkPeerCountUpdate(e.NetworkID, 1)
			case event.NetworkPeerClose:
				t.handleNetworkPeerCountUpdate(e.NetworkID, -1)
			}
		case <-ctx.Done():
			return
		}

		t.scheduleCertRenewal()
	}
}

func (t *control) handlePeerAdd(ctx context.Context, peerID uint64) {
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
		t.observers.EmitLocal(event.NetworkNegotiationComplete{})
	}()
}

func (t *control) handlePeerBinding(ctx context.Context, peerID uint64, networkKeys [][]byte) {
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

		n, ok := t.networks[c.networkID]
		if !ok {
			continue
		}

		go func() {
			if err := t.renewCertificateWithPeer(ctx, n.network, peer); err != nil {
				t.logger.Debug(
					"certificate renew via peer failed",
					zap.Stringer("peer", peer.vnicPeer.HostID()),
					logutil.ByteHex("network", dao.NetworkKey(n.network)),
					zap.Error(err),
				)
			}
		}()
	}
}

func (t *control) handleNetworkCertUpdate(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	networkKey := dao.NetworkKey(network)
	for _, peer := range t.peers {
		if peer.hasNetworkBinding(networkKey) {
			go peer.sendCertificateUpdate(network)
		}
	}
}

func (t *control) handleNetworkAdd(ctx context.Context) {
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

func (t *control) handleNetworkPeerCountUpdate(networkID uint64, d int) {
	t.lock.Lock()
	n, ok := t.networks[networkID]
	if !ok {
		t.lock.Unlock()
		return
	}

	n.peerCount += d
	peerCount := n.peerCount
	t.lock.Unlock()

	t.observers.EmitLocal(event.NetworkPeerCountUpdate{
		NetworkID: networkID,
		PeerCount: peerCount,
	})
}

func (t *control) dispatchCertificateRenewNotification(network *networkv1.Network, renewErr error) {
	var notification *notificationv1.Notification
	var err error

	if renewErr != nil {
		notification, err = dao.NewNotification(
			t.store,
			notificationv1.Notification_STATUS_ERROR,
			"Certificate renewal failed",
			dao.WithNotificationMessage(renewErr.Error()),
			dao.WithNotificationSubject(
				notificationv1.Notification_Subject_NOTIFICATION_SUBJECT_MODEL_NETWORK,
				network.Id,
			),
		)
	} else {
		notification, err = dao.NewNotification(
			t.store,
			notificationv1.Notification_STATUS_SUCCESS,
			"Certificate renewed",
			dao.WithNotificationSubject(
				notificationv1.Notification_Subject_NOTIFICATION_SUBJECT_MODEL_NETWORK,
				network.Id,
			),
		)
	}
	if err != nil {
		t.logger.Debug(
			"creating notification failed",
			zap.Error(err),
		)
		return
	}
	t.notification.Dispatch(notification)
}

type certificateRenewFunc func(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error)

// renewCertificateWithRenewFunc ...
func (t *control) renewCertificateWithRenewFunc(ctx context.Context, network *networkv1.Network, fn certificateRenewFunc) error {
	subject := t.profile.Name
	if network.Alias != "" {
		subject = network.Alias
	}

	csr, err := dao.NewCertificateRequest(
		t.profile.Key,
		certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN,
		dao.WithSubject(subject),
	)
	if err != nil {
		go t.dispatchCertificateRenewNotification(network, err)
		return err
	}

	cert, err := fn(ctx, network.Certificate, csr)
	if err != nil {
		go t.dispatchCertificateRenewNotification(network, err)
		return err
	}
	if err := dao.VerifyCertificate(cert); err != nil {
		go t.dispatchCertificateRenewNotification(network, err)
		return err
	}

	if err := t.setCertificate(network.Id, cert); err != nil {
		go t.dispatchCertificateRenewNotification(network, err)
		return err
	}

	go t.dispatchCertificateRenewNotification(network, nil)
	return nil
}

// renewCertificate ...
func (t *control) renewCertificate(ctx context.Context, network *networkv1.Network) error {
	if config := network.GetServerConfig(); config != nil {
		return t.renewCertificateWithRenewFunc(
			ctx,
			network,
			func(ctx context.Context, _ *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
				return dao.SignCertificateRequestWithNetwork(csr, config)
			},
		)
	}

	return t.renewCertificateWithRenewFunc(
		ctx,
		network,
		func(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
			networkKey := dao.NetworkKey(network)
			client, err := t.dialer.Client(ctx, networkKey, networkKey, AddressSalt)
			if err != nil {
				return nil, err
			}
			caClient := networkv1ca.NewCAClient(client)

			renewReq := &networkv1ca.CARenewRequest{
				Certificate:        cert,
				CertificateRequest: csr,
			}
			renewRes := &networkv1ca.CARenewResponse{}
			if err := caClient.Renew(ctx, renewReq, renewRes); err != nil {
				return nil, err
			}

			return renewRes.Certificate, nil
		},
	)
}

func (t *control) renewCertificateWithPeer(ctx context.Context, network *networkv1.Network, peer *peer) error {
	return t.renewCertificateWithRenewFunc(
		ctx,
		network,
		func(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
			req := &networkv1ca.CAPeerRenewRequest{
				Certificate:        cert,
				CertificateRequest: csr,
			}
			res := &networkv1ca.CAPeerRenewResponse{}
			if err := peer.client.CA().Renew(ctx, req, res); err != nil {
				return nil, err
			}
			return res.Certificate, nil
		},
	)
}

// AddPeer ...
func (t *control) AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer {
	p := newPeer(id, vnicPeer, client, t.logger, t.observers, t.broker, t.vpn, t.qosc, t.certificates)

	t.lock.Lock()
	defer t.lock.Unlock()

	t.peers[p.id] = p

	return p
}

// RemovePeer ...
func (t *control) RemovePeer(id uint64) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.peers[id].close()
	delete(t.peers, id)
}

func (t *control) startNetworks() {
	t.lock.Lock()
	defer t.lock.Unlock()

	networks, err := dao.GetNetworks(t.store)
	if err != nil {
		t.logger.Fatal("loading networks failed", zap.Error(err))
	}

	for _, n := range networks {
		cert := dao.CertificateRoot(n.Certificate)

		if _, err := t.vpn.AddNetwork(n.Certificate); err != nil {
			t.logger.Error(
				"starting network failed",
				zap.String("name", cert.Subject),
				logutil.ByteHex("key", cert.Key),
				zap.Error(err),
			)
			continue
		}

		t.networks[n.Id] = &network{network: n}
		t.certificates.Insert(n)
		t.dialer.replaceOrInsertNetwork(n)

		t.logger.Info(
			"network started",
			zap.String("name", cert.Subject),
			logutil.ByteHex("key", cert.Key),
		)

		t.observers.EmitLocal(event.NetworkStart{Network: n})
	}

	t.certificates.SetLoaded()
}

func (t *control) scheduleCertRenewal() {
	t.lock.Lock()
	defer t.lock.Unlock()

	minNextTime := timeutil.MaxTime

	for _, n := range t.networks {
		nextTime := nextCertificateRenewTime(n.network)
		if nextTime.Before(minNextTime) {
			minNextTime = nextTime
		}
	}

	now := timeutil.Now()

	if minNextTime.Before(now) {
		minNextTime = now
	}
	if minNextTime.Before(t.nextCertRenewTime) {
		minNextTime = t.nextCertRenewTime
	}

	t.certRenewTimeout.Reset(minNextTime.Sub(now))
}

func (t *control) renewExpiredCerts(ctx context.Context) {
	t.lock.Lock()
	defer t.lock.Unlock()

	now := timeutil.Now()
	t.nextCertRenewTime = now.Add(certRecheckInterval)

	for _, n := range t.networks {
		n := n

		if now.After(nextCertificateRenewTime(n.network)) || isCertificateSubjectMismatched(n.network) {
			go func() {
				if err := t.renewCertificate(ctx, n.network); err != nil {
					t.logger.Debug(
						"network certificate renewal failed",
						logutil.ByteHex("network", dao.NetworkKey(n.network)),
						zap.Error(err),
					)
				}
			}()
		}
	}
}

func (t *control) mutateNetworkWithFinalizer(id uint64, mutate func(*networkv1.Network) error, finalize func(*networkv1.Network)) error {
	t.lock.Lock()
	defer t.lock.Unlock()

	n, ok := t.networks[id]
	if !ok {
		return ErrNetworkNotFound
	}

	clone := proto.Clone(n.network).(*networkv1.Network)
	if err := mutate(clone); err != nil {
		return err
	}

	if err := dao.UpsertNetwork(t.store, clone); err != nil {
		return err
	}

	t.networks[id].network = clone

	finalize(clone)
	return nil
}

func noopMutateNetworkFinalizer(*networkv1.Network) {}

func (t *control) mutateNetwork(id uint64, mutate func(*networkv1.Network) error) error {
	return t.mutateNetworkWithFinalizer(id, mutate, noopMutateNetworkFinalizer)
}

func (t *control) setCertificate(id uint64, cert *certificate.Certificate) error {
	return t.mutateNetworkWithFinalizer(
		id,
		func(network *networkv1.Network) error {
			network.Certificate = cert
			return nil
		},
		func(network *networkv1.Network) {
			t.certificates.Insert(network)
			t.dialer.replaceOrInsertNetwork(network)
			t.observers.EmitGlobal(event.NetworkCertUpdate{Network: proto.Clone(network).(*networkv1.Network)})
		},
	)
}

func (t *control) setNetworkAlias(id uint64, name string) error {
	return t.mutateNetwork(id, func(network *networkv1.Network) error {
		network.Alias = name
		return nil
	})
}

// Certificate ...
func (t *control) Certificate(networkKey []byte) (*certificate.Certificate, bool) {
	if ci, ok := t.certificates.Get(networkKey); ok {
		return ci.certificate, true
	}
	return nil, false
}

// Add ...
func (t *control) Add(n *networkv1.Network, certLogs []*networkv1ca.CertificateLog) error {
	t.lock.Lock()
	defer t.lock.Unlock()

	if _, ok := t.networks[n.Id]; ok {
		return errors.New("duplicate network id")
	}

	if _, err := t.vpn.AddNetwork(n.Certificate); err != nil {
		return err
	}

	err := t.store.Update(func(tx kv.RWTx) error {
		for _, l := range certLogs {
			if err := dao.InsertCertificateLog(tx, l); err != nil {
				return err
			}
		}

		order, err := dao.NextNetworkDisplayOrder(tx)
		if err != nil {
			return err
		}

		n.DisplayOrder = order
		return dao.UpsertNetwork(tx, n)
	})
	if err != nil {
		return err
	}

	t.networks[n.Id] = &network{network: n}
	t.certificates.Insert(n)
	t.dialer.replaceOrInsertNetwork(n)

	t.observers.EmitGlobal(event.NetworkAdd{Network: n})
	t.observers.EmitLocal(event.NetworkStart{Network: n})

	return nil
}

// Remove ...
func (t *control) Remove(id uint64) error {
	t.lock.Lock()
	defer t.lock.Unlock()

	n, ok := t.networks[id]
	if !ok {
		return ErrNetworkNotFound
	}
	networkKey := dao.NetworkKey(n.network)

	if err := t.vpn.RemoveNetwork(dao.NetworkKey(n.network)); err != nil {
		return err
	}

	err := t.store.Update(func(tx kv.RWTx) error {
		if err := dao.DeleteCertificateLogByNetwork(tx, id); err != nil {
			return err
		}
		return dao.DeleteNetwork(tx, id)
	})
	if err != nil {
		return err
	}

	for _, p := range t.peers {
		p.closeNetwork(networkKey)
	}

	t.dialer.removeNetwork(n.network)
	t.certificates.Delete(networkKey)
	delete(t.networks, id)

	t.observers.EmitLocal(event.NetworkStop{Network: n.network})
	t.observers.EmitGlobal(event.NetworkRemove{Network: n.network})

	return nil
}

func (t *control) ReadEvents(ctx context.Context) <-chan *networkv1.NetworkEvent {
	ch := make(chan *networkv1.NetworkEvent, 8)

	go func() {
		t.lock.Lock()
		for _, n := range t.networks {
			ch <- &networkv1.NetworkEvent{
				Body: &networkv1.NetworkEvent_NetworkStart_{
					NetworkStart: &networkv1.NetworkEvent_NetworkStart{
						Network:   n.network,
						PeerCount: uint32(n.peerCount),
					},
				},
			}
		}
		t.lock.Unlock()

		events := make(chan interface{}, 8)
		t.observers.Notify(events)
		defer t.observers.StopNotifying(events)

		for {
			select {
			case e := <-events:
				switch e := e.(type) {
				case event.NetworkStart:
					ch <- &networkv1.NetworkEvent{
						Body: &networkv1.NetworkEvent_NetworkStart_{
							NetworkStart: &networkv1.NetworkEvent_NetworkStart{
								Network: e.Network,
							},
						},
					}
				case event.NetworkStop:
					ch <- &networkv1.NetworkEvent{
						Body: &networkv1.NetworkEvent_NetworkStop_{
							NetworkStop: &networkv1.NetworkEvent_NetworkStop{
								NetworkId: e.Network.Id,
							},
						},
					}
				case event.NetworkPeerCountUpdate:
					ch <- &networkv1.NetworkEvent{
						Body: &networkv1.NetworkEvent_NetworkPeerCountUpdate_{
							NetworkPeerCountUpdate: &networkv1.NetworkEvent_NetworkPeerCountUpdate{
								NetworkId: e.NetworkID,
								PeerCount: uint32(e.PeerCount),
							},
						},
					}
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	return ch
}

func (t *control) UpdateDisplayOrder(ids []uint64) error {
	t.lock.Lock()
	defer t.lock.Unlock()

	if len(ids) != len(t.networks) {
		return errors.New("network id list incomplete")
	}

	return t.store.Update(func(tx kv.RWTx) error {
		for i, id := range ids {
			n, ok := t.networks[id]
			if !ok {
				return ErrNetworkNotFound
			}

			if n.network.DisplayOrder == uint32(i) {
				continue
			}

			clone := proto.Clone(n.network).(*networkv1.Network)
			clone.DisplayOrder = uint32(i)

			if err := dao.UpsertNetwork(tx, clone); err != nil {
				return err
			}

			t.networks[id].network = clone
		}
		return nil
	})
}
