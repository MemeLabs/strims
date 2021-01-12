package ca

import (
	"context"
	"errors"
	"sync"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/services/ca"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

var ErrNetworkNotFound = errors.New("network not found")

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers, dialer *dialer.Control) *Control {
	events := make(chan interface{}, 128)
	observers.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		observers: observers,
		events:    events,
		dialer:    dialer,
		servers:   map[uint64]context.CancelFunc{},
	}
}

// Control ...
type Control struct {
	logger            *zap.Logger
	vpn               *vpn.Host
	observers         *event.Observers
	lock              sync.Mutex
	servers           map[uint64]context.CancelFunc
	events            chan interface{}
	dialer            *dialer.Control
	certRenewTimeout  <-chan time.Time
	lastCertRenewTime time.Time
}

// Run ...
func (t *Control) Run(ctx context.Context) {
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

func (t *Control) handleNetworkStart(ctx context.Context, network *networkv1.Network) {
	if network.Key == nil {
		return
	}

	t.logger.Info(
		"starting certificate authority",
		logutil.ByteHex("network", network.Key.Public),
	)

	server, err := t.dialer.Server(network.Key.Public, network.Key, ca.AddressSalt)
	if err != nil {
		t.logger.Error(
			"starting certificate authority failed",
			logutil.ByteHex("network", network.Key.Public),
			zap.Error(err),
		)
		return
	}

	networkv1ca.RegisterCAService(server, ca.NewService(t.logger, network))
	ctx, cancel := context.WithCancel(ctx)
	go server.Listen(ctx)

	t.servers[network.Id] = cancel
}

func (t *Control) handleNetworkStop(network *networkv1.Network) {
	if server, ok := t.servers[network.Id]; ok {
		server()
	}
}

// ForwardRenewRequest ...
func (t *Control) ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
	networkKey := networkKeyForCertificate(cert)
	client, err := t.dialer.Client(networkKey, networkKey, ca.AddressSalt)
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

func networkKeyForCertificate(cert *certificate.Certificate) []byte {
	return dao.GetRootCert(cert).Key
}
