package ca

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

var ErrNetworkNotFound = errors.New("network not found")

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, profile *pb.Profile, observers *event.Observers) *Control {
	events := make(chan interface{}, 128)
	observers.CA.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		profile:   profile,
		observers: observers,
		events:    events,
		servers:   map[uint64]*Server{},
	}
}

// Control ...
type Control struct {
	logger            *zap.Logger
	vpn               *vpn.Host
	profile           *pb.Profile
	observers         *event.Observers
	lock              sync.Mutex
	servers           map[uint64]*Server
	events            chan interface{}
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
				t.startService(ctx, e.Network)
			case event.NetworkStop:
				t.stopService(e.Network)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (t *Control) startService(ctx context.Context, network *pb.Network) {
	if network.Key == nil {
		return
	}

	client, ok := t.vpn.Client(network.Key.Public)
	if !ok {
		return
	}

	ca, err := NewServer(ctx, t.logger, client, network)
	if err != nil {
		t.logger.Debug("ca start error", zap.Error(err))
	}

	t.servers[network.Id] = ca
}

func (t *Control) stopService(network *pb.Network) {
	server, ok := t.servers[network.Id]
	if !ok {
		return
	}
	server.Close()
}

func networkKeyForCertificate(cert *pb.Certificate) []byte {
	return dao.GetRootCert(cert).Key
}
