package bootstrap

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/api"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

type Control interface {
	Run(ctx context.Context)
	AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer
	RemovePeer(id uint64)
	PublishingEnabled() bool
	Publish(ctx context.Context, peerID uint64, network *network.Network, validDuration time.Duration) error
}

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers) Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	return &control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		events:    events,
		peers:     map[uint64]*peer{},
	}
}

// Control ...
type control struct {
	logger *zap.Logger
	vpn    *vpn.Host
	store  *dao.ProfileStore

	lock              sync.Mutex
	observers         *event.Observers
	events            chan interface{}
	certRenewTimeout  <-chan time.Time
	lastCertRenewTime time.Time
	nextID            uint64
	peers             map[uint64]*peer
}

// Run ...
func (t *control) Run(ctx context.Context) {
	go t.startClients()

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.PeerAdd:
				t.handlePeerAdd(ctx, e.ID)
			case event.NetworkBootstrapClientAdd:
				go t.startClient(e.Client)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (t *control) handlePeerAdd(ctx context.Context, id uint64) {
	peer, ok := t.peers[id]
	if !ok {
		return
	}

	go func() {
		var res networkv1bootstrap.BootstrapPeerGetPublishEnabledResponse
		if err := peer.client.Bootstrap().GetPublishEnabled(ctx, &networkv1bootstrap.BootstrapPeerGetPublishEnabledRequest{}, &res); err != nil {
			t.logger.Debug("bootstrap publish enabled check failed", zap.Error(err))
		}

		// TODO: locking...
		peer.PublishEnabled = res.Enabled
	}()
}

// AddPeer ...
func (t *control) AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer {
	p := &peer{
		vnicPeer: vnicPeer,
		client:   client,
	}

	t.lock.Lock()
	defer t.lock.Unlock()

	t.peers[id] = p

	return p
}

// RemovePeer ...
func (t *control) RemovePeer(id uint64) {
	t.lock.Lock()
	defer t.lock.Unlock()

	delete(t.peers, id)
}

func (t *control) startClients() {
	clients, err := dao.BootstrapClients.GetAll(t.store)
	if err != nil {
		t.logger.Fatal("loading bootstrap clients failed", zap.Error(err))
	}

	for _, client := range clients {
		go t.startClient(client)
	}
}

func (t *control) startClient(client *networkv1bootstrap.BootstrapClient) {
	var err error
	switch client := client.ClientOptions.(type) {
	case *networkv1bootstrap.BootstrapClient_WebsocketOptions:
		err = t.vpn.VNIC().Dial(vnic.WebSocketAddr{
			URL:                   client.WebsocketOptions.Url,
			InsecureSkipVerifyTLS: client.WebsocketOptions.InsecureSkipVerifyTls,
		})
	}

	if err != nil {
		t.logger.Debug("starting bootstrap client failed", zap.Error(err))
	}
}

// PublishingEnabled ...
func (t *control) PublishingEnabled() bool {
	// TODO: load from db
	return true
}

// Publish ...
func (t *control) Publish(ctx context.Context, peerID uint64, network *network.Network, validDuration time.Duration) error {
	peer, ok := t.peers[peerID]
	if !ok {
		return errors.New("peer id not found")
	}

	if !peer.PublishEnabled {
		return errors.New("peer does not support network bootstrapping")
	}

	config := network.GetServerConfig()
	if config == nil {
		return errors.New("only managed networks can be published")
	}

	networkCert, err := dao.NewNetworkCertificate(config)
	if err != nil {
		return err
	}
	csr := &certificate.CertificateRequest{
		Key:      peer.vnicPeer.Certificate.Key,
		KeyType:  peer.vnicPeer.Certificate.KeyType,
		KeyUsage: certificate.KeyUsage_KEY_USAGE_BROKER | certificate.KeyUsage_KEY_USAGE_SIGN,
	}
	cert, err := dao.SignCertificateRequest(csr, validDuration, config.Key)
	if err != nil {
		return err
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: networkCert}

	return peer.client.Bootstrap().Publish(ctx, &networkv1bootstrap.BootstrapPeerPublishRequest{Certificate: cert}, &networkv1bootstrap.BootstrapPeerPublishResponse{})
}
