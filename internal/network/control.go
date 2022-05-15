package network

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/api"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network/ca"
	"github.com/MemeLabs/go-ppspp/internal/network/dialer"
	"github.com/MemeLabs/go-ppspp/internal/notification"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	networkv1errors "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/errors"
	notificationv1 "github.com/MemeLabs/go-ppspp/pkg/apis/notification/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
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

type network struct {
	network   *networkv1.Network
	peerCount int
}

type Control interface {
	CA() CA
	Dialer() Dialer
	Run()
	AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer
	RemovePeer(id uint64)
	Certificate(networkKey []byte) (*certificate.Certificate, bool)
	Add(network *networkv1.Network) error
	Remove(id uint64) error
	SetAlias(id uint64, name string) error
	ReadEvents(ctx context.Context) <-chan *networkv1.NetworkEvent
}

// Broker negotiates common networks with peers.
type Broker interface {
	SendKeys(c ioutil.ReadWriteFlusher, keys [][]byte) error
	ReceiveKeys(c ioutil.ReadWriteFlusher, keys [][]byte) ([][]byte, error)
}

var _ CA = (*ca.CA)(nil)

type CA interface {
	ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error)
	FindBySubject(ctx context.Context, networkKey []byte, subject string) (*certificate.Certificate, error)
	FindByKey(ctx context.Context, networkKey []byte, key []byte) (*certificate.Certificate, error)
}

var _ Dialer = (*dialer.Dialer)(nil)

type Dialer interface {
	ServerDialer(ctx context.Context, networkKey []byte, port uint16, publisher dialer.HostAddrPublisher) (rpc.Dialer, error)
	Server(ctx context.Context, networkKey []byte, key *key.Key, salt []byte) (*rpc.Server, error)
	ServerWithHostAddr(ctx context.Context, networkKey []byte, port uint16) (*rpc.Server, error)
	ClientDialer(ctx context.Context, networkKey []byte, resolver dialer.HostAddrResolver) (rpc.Dialer, error)
	Client(ctx context.Context, networkKey, key, salt []byte) (*dialer.RPCClient, error)
	ClientWithHostAddr(ctx context.Context, networkKey []byte, hostID kademlia.ID, port uint16) (*dialer.RPCClient, error)
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	transfer transfer.Control,
	broker Broker,
	profile *profilev1.Profile,
	notification notification.Control,
) Control {
	d := dialer.NewDialer(logger, vpn, profile.Key)

	return &control{
		ctx:          ctx,
		logger:       logger,
		vpn:          vpn,
		store:        store,
		observers:    observers,
		broker:       broker,
		profile:      profile,
		notification: notification,

		events:           observers.Chan(),
		qosc:             vpn.VNIC().QOS().AddClass(1),
		ca:               ca.NewCA(ctx, logger, store, observers, d, transfer),
		dialer:           d,
		certRenewTimeout: time.NewTimer(0),
		networks:         map[uint64]*network{},
		peers:            map[uint64]*peer{},
		certificates:     newCertificateMap(),
	}
}

// Control ...
type control struct {
	ctx          context.Context
	logger       *zap.Logger
	vpn          *vpn.Host
	store        *dao.ProfileStore
	observers    *event.Observers
	broker       Broker
	profile      *profilev1.Profile
	notification notification.Control

	events            chan any
	qosc              *qos.Class
	lock              sync.Mutex
	ca                *ca.CA
	dialer            *dialer.Dialer
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
func (t *control) Run() {
	go t.ca.Run()

	t.startNetworks()
	t.scheduleCertRenewal()

	for {
		select {
		case <-t.certRenewTimeout.C:
			t.renewExpiredCerts()
			t.scheduleCertRenewal()
		case e := <-t.events:
			switch e := e.(type) {
			case event.PeerAdd:
				t.handlePeerAdd(e.ID)
			case event.NetworkPeerBindings:
				t.handlePeerBinding(e.PeerID, e.NetworkKeys)
			case event.NetworkPeerOpen:
				t.handleNetworkPeerCountUpdate(e.NetworkID, 1)
			case event.NetworkPeerClose:
				t.handleNetworkPeerCountUpdate(e.NetworkID, -1)
			case *networkv1.NetworkChangeEvent:
				t.handleNetworkChange(e.Network)
				t.scheduleCertRenewal()
			case *networkv1.NetworkDeleteEvent:
				t.handleNetworkDelete(e.Network)
			}
		case <-t.ctx.Done():
			return
		}
	}
}

func (t *control) startNetworks() error {
	t.lock.Lock()
	defer t.lock.Unlock()

	networks, err := dao.Networks.GetAll(t.store)
	if err != nil {
		t.logger.Fatal("loading networks failed", zap.Error(err))
	}

	for _, n := range networks {
		if err := t.startNetwork(n); err != nil {
			return err
		}
	}

	t.certificates.SetLoaded()

	return nil
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

func (t *control) handlePeerAdd(peerID uint64) {
	t.lock.Lock()
	defer t.lock.Unlock()

	peer, ok := t.peers[peerID]
	if !ok {
		return
	}

	go func() {
		if err := peer.negotiateNetworks(t.ctx); err != nil {
			t.logger.Debug("network negotiation failed", zap.Error(err))
		}
		t.observers.EmitLocal(event.NetworkNegotiationComplete{})
	}()
}

func (t *control) handlePeerBinding(peerID uint64, networkKeys [][]byte) {
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
			if err := t.renewCertificateWithPeer(t.ctx, n.network, peer); err != nil {
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

func (t *control) handleNetworkChange(n *networkv1.Network) error {
	t.lock.Lock()
	defer t.lock.Unlock()
	return t.startNetwork(n)
}

func (t *control) handleNetworkDelete(n *networkv1.Network) error {
	t.lock.Lock()
	defer t.lock.Unlock()
	return t.stopNetwork(n)
}

func (t *control) startNetwork(n *networkv1.Network) error {
	if nn, ok := t.networks[n.Id]; ok {
		if !proto.Equal(nn.network.Certificate, n.Certificate) {
			t.certificates.Insert(n)
			t.dialer.ReplaceOrInsertNetwork(n)

			for _, peer := range t.peers {
				if peer.hasNetworkBinding(dao.NetworkKey(n)) {
					go peer.sendCertificateUpdate(n)
				}
			}
		}
		return nil
	}

	if _, err := t.vpn.AddNetwork(dao.NetworkKey(n)); err != nil {
		return err
	}

	t.networks[n.Id] = &network{network: n}
	t.certificates.Insert(n)
	t.dialer.ReplaceOrInsertNetwork(n)

	t.observers.EmitLocal(event.NetworkStart{Network: n})

	// TODO: throttle
	for _, peer := range t.peers {
		peer := peer
		go func() {
			if err := peer.negotiateNetworks(t.ctx); err != nil {
				t.logger.Debug("network negotiation failed", zap.Error(err))
			}
		}()
	}

	return nil
}

func (t *control) stopNetwork(n *networkv1.Network) error {
	if err := t.vpn.RemoveNetwork(dao.NetworkKey(n)); err != nil {
		return err
	}

	for _, p := range t.peers {
		p.closeNetwork(dao.NetworkKey(n))
	}

	t.dialer.RemoveNetwork(n)
	t.certificates.Delete(dao.NetworkKey(n))
	delete(t.networks, n.Id)

	t.observers.EmitLocal(event.NetworkStop{Network: n})

	return nil
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
	cert, err := t.renewCertificateWithRenewFunc1(ctx, network, fn)

	errCode := networkv1errors.ErrorCode(rpc.ErrorCode(err))
	if err != nil && errCode == networkv1errors.ErrorCode_UNDEFINED {
		errCode = networkv1errors.ErrorCode_UNKNOWN
	}

	go t.dispatchCertificateRenewNotification(network, err)

	_, err = dao.Networks.Transform(t.store, network.Id, func(p *networkv1.Network) error {
		p.CertificateRenewalError = errCode
		if cert != nil {
			p.Certificate = cert
		}
		return nil
	})
	return err
}

func (t *control) renewCertificateWithRenewFunc1(ctx context.Context, network *networkv1.Network, fn certificateRenewFunc) (*certificate.Certificate, error) {
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
		return nil, err
	}

	cert, err := fn(ctx, network.Certificate, csr)
	if err != nil {
		return nil, err
	}
	if err := dao.VerifyCertificate(cert); err != nil {
		return nil, err
	}

	return cert, nil
}

// renewCertificate ...
func (t *control) renewCertificate(ctx context.Context, network *networkv1.Network) error {
	return t.renewCertificateWithRenewFunc(
		ctx,
		network,
		func(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
			networkKey := dao.NetworkKey(network)
			client, err := t.dialer.Client(ctx, networkKey, networkKey, ca.AddressSalt)
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

func (t *control) renewExpiredCerts() {
	t.lock.Lock()
	defer t.lock.Unlock()

	now := timeutil.Now()
	t.nextCertRenewTime = now.Add(certRecheckInterval)

	for _, n := range t.networks {
		n := n

		if now.After(nextCertificateRenewTime(n.network)) || isCertificateSubjectMismatched(n.network) {
			go func() {
				if err := t.renewCertificate(t.ctx, n.network); err != nil {
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

func (t *control) SetAlias(id uint64, alias string) error {
	var aliasChanged bool
	network, err := dao.Networks.Transform(t.store, id, func(p *networkv1.Network) error {
		aliasChanged = p.Alias != alias
		p.Alias = alias
		return nil
	})
	if err != nil {
		return err
	}

	if aliasChanged {
		// TODO: defer via renew scheduler...
		go t.renewCertificate(context.Background(), network)
	}

	return err
}

// Certificate ...
func (t *control) Certificate(networkKey []byte) (*certificate.Certificate, bool) {
	if ci, ok := t.certificates.Get(networkKey); ok {
		return ci.certificate, true
	}
	return nil, false
}

func (t *control) Add(network *networkv1.Network) error {
	var logs []*networkv1ca.CertificateLog
	if network.GetServerConfig() != nil {
		for c := network.Certificate; c != nil; c = c.GetParent() {
			log, err := dao.NewCertificateLog(t.store, network.Id, c)
			if err != nil {
				return err
			}
			logs = append(logs, log)
		}
	}

	adminRecord, err := dao.NewDirectoryUserRecord(t.store, network.Id, t.profile.Key.Public)
	if err != nil {
		return err
	}

	return t.store.Update(func(tx kv.RWTx) error {
		if err := dao.Networks.Insert(tx, network); err != nil {
			return err
		}
		for _, log := range logs {
			if err := dao.CertificateLogs.Insert(tx, log); err != nil {
				return err
			}
		}
		return dao.DirectoryUserRecords.Insert(tx, adminRecord)
	})
}

func (t *control) Remove(id uint64) error {
	return dao.Networks.Delete(t.store, id)
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

		events, done := t.observers.Events()
		defer done()

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
				case *networkv1.UIConfigChangeEvent:
					ch <- &networkv1.NetworkEvent{
						Body: &networkv1.NetworkEvent_UiConfigUpdate{
							UiConfigUpdate: e.UiConfig,
						},
					}
				case *networkv1.NetworkChangeEvent:
					ch <- &networkv1.NetworkEvent{
						Body: &networkv1.NetworkEvent_NetworkUpdate{
							NetworkUpdate: e.Network,
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
