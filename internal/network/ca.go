package network

import (
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// var ErrNetworkNotFound = errors.New("network not found")

type CA interface {
	ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error)
}

// newCA ...
func newCA(logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer Dialer,
) *ca {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	return &ca{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		events:    events,
		dialer:    dialer,
		servers:   map[uint64]context.CancelFunc{},
	}
}

// CA ...
type ca struct {
	logger            *zap.Logger
	vpn               *vpn.Host
	store             *dao.ProfileStore
	observers         *event.Observers
	lock              sync.Mutex
	servers           map[uint64]context.CancelFunc
	events            chan interface{}
	dialer            Dialer
	certRenewTimeout  <-chan time.Time
	lastCertRenewTime time.Time
}

// Run ...
func (t *ca) Run(ctx context.Context) {
	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.NetworkStart:
				t.handleNetworkStart(ctx, e.Network)
			case event.NetworkStop:
				t.handleNetworkStop(e.Network)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (t *ca) handleNetworkStart(ctx context.Context, network *networkv1.Network) {
	config := network.GetServerConfig()
	if config == nil {
		return
	}

	t.logger.Info(
		"starting certificate authority",
		logutil.ByteHex("network", config.Key.Public),
	)

	server, err := t.dialer.Server(config.Key.Public, config.Key, AddressSalt)
	if err != nil {
		t.logger.Error(
			"starting certificate authority failed",
			logutil.ByteHex("network", config.Key.Public),
			zap.Error(err),
		)
		return
	}

	networkv1ca.RegisterCAService(server, newCAService(t.logger, t.store, network))
	ctx, cancel := context.WithCancel(ctx)
	go server.Listen(ctx)

	t.servers[network.Id] = cancel
}

func (t *ca) handleNetworkStop(network *networkv1.Network) {
	if server, ok := t.servers[network.Id]; ok {
		server()
	}
}

// ForwardRenewRequest ...
func (t *ca) ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
	networkKey := networkKeyForCertificate(cert)
	client, err := t.dialer.Client(networkKey, networkKey, AddressSalt)
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
}
