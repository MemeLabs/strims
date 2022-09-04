// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bootstrap

import (
	"context"
	"errors"
	"net/url"
	"time"

	"github.com/MemeLabs/strims/internal/api"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	network "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"github.com/avast/retry-go"
	"go.uber.org/zap"
)

const (
	dialRetryLimit        = 10
	dialRetryInitialDelay = time.Second
)

type Control interface {
	Run()
	AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer
	RemovePeer(id uint64)
	PublishingEnabled() bool
	Publish(ctx context.Context, peerID uint64, network *network.Network, validDuration time.Duration) error
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
) Control {
	return &control{
		ctx:    ctx,
		logger: logger,
		vpn:    vpn,
		store:  store,

		events: observers.Chan(),
	}
}

// Control ...
type control struct {
	ctx    context.Context
	logger *zap.Logger
	vpn    *vpn.Host
	store  *dao.ProfileStore

	events            chan any
	certRenewTimeout  <-chan time.Time
	lastCertRenewTime time.Time
	nextID            uint64
	peers             syncutil.Map[uint64, *peer]
	peerHostClientIDs syncutil.Map[kademlia.ID, uint64]
}

// Run ...
func (t *control) Run() {
	go t.startClients()

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.PeerAdd:
				go t.handlePeerAdd(e.ID)
			case event.PeerRemove:
				go t.handlePeerRemove(e.HostID)
			case *networkv1bootstrap.BootstrapClientChange:
				go t.startClient(e.BootstrapClient)
			}
		case <-t.ctx.Done():
			return
		}
	}
}

func (t *control) handlePeerAdd(id uint64) {
	peer, ok := t.peers.Get(id)
	if !ok {
		return
	}

	var res networkv1bootstrap.BootstrapPeerGetPublishEnabledResponse
	if err := peer.client.Bootstrap().GetPublishEnabled(t.ctx, &networkv1bootstrap.BootstrapPeerGetPublishEnabledRequest{}, &res); err != nil {
		t.logger.Debug("bootstrap publish enabled check failed", zap.Error(err))
	}

	peer.PublishEnabled.Store(res.Enabled)
}

func (t *control) handlePeerRemove(hostID kademlia.ID) {
	if id, ok := t.peerHostClientIDs.GetAndDelete(hostID); ok {
		t.logger.Info("reconnecting to bootstrap client")

		client, err := dao.BootstrapClients.Get(t.store, id)
		if err != nil {
			t.logger.Warn("loading bootstrap client failed", zap.Error(err))
		} else {
			t.startClient(client)
		}
	}
}

// AddPeer ...
func (t *control) AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer {
	p := &peer{
		vnicPeer: vnicPeer,
		client:   client,
	}

	t.peers.Set(id, p)

	return p
}

// RemovePeer ...
func (t *control) RemovePeer(id uint64) {
	t.peers.Delete(id)
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
	var peer *vnic.Peer
	err := retry.Do(
		func() (err error) {
			switch opt := client.ClientOptions.(type) {
			case *networkv1bootstrap.BootstrapClient_WebsocketOptions:
				peer, err = t.startWSClient(opt.WebsocketOptions)
			}
			return err
		},
		retry.RetryIf(func(err error) bool {
			var peerInitErr *vnic.PeerInitError
			return !errors.As(err, &peerInitErr)
		}),
		retry.Attempts(dialRetryLimit),
		retry.Context(t.ctx),
		retry.Delay(dialRetryInitialDelay),
		retry.DelayType(retry.BackOffDelay),
	)

	if err != nil {
		t.logger.Warn("starting bootstrap client failed", zap.Error(err))
	} else {
		t.peerHostClientIDs.Set(peer.HostID(), client.Id)
	}
}

func (t *control) startWSClient(opt *networkv1bootstrap.BootstrapClientWebSocketOptions) (*vnic.Peer, error) {
	u, err := url.Parse(opt.Url)
	if err != nil {
		return nil, err
	}
	if opt.InsecureSkipVerifyTls {
		u.Fragment = "insecure"
	}
	return t.vpn.VNIC().Dial(u.String())
}

// PublishingEnabled ...
func (t *control) PublishingEnabled() bool {
	// TODO: load from db
	return true
}

// Publish ...
func (t *control) Publish(ctx context.Context, peerID uint64, network *network.Network, validDuration time.Duration) error {
	peer, ok := t.peers.Get(peerID)
	if !ok {
		return errors.New("peer id not found")
	}

	if !peer.PublishEnabled.Load() {
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
