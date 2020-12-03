package directory

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers, dialer *dialer.Control) *Control {
	events := make(chan interface{}, 128)
	observers.Network.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		observers: observers,
		events:    events,
		dialer:    dialer,
	}
}

// Control ...
type Control struct {
	logger            *zap.Logger
	vpn               *vpn.Host
	observers         *event.Observers
	events            chan interface{}
	dialer            *dialer.Control
	servers           sync.Map
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

func (t *Control) handleNetworkStart(ctx context.Context, network *pb.Network) {
	if network.Key == nil {
		return
	}

	// TODO: locking

	go func() {
		t.logger.Info(
			"starting directory service",
			logutil.ByteHex("network", network.Key.Public),
		)

		if err := t.startServer(ctx, network); err != nil {
			t.logger.Error(
				"starting directory service failed",
				logutil.ByteHex("network", network.Key.Public),
				zap.Error(err),
			)
		}
	}()

}

func (t *Control) startServer(ctx context.Context, network *pb.Network) error {
	server, err := t.dialer.Server(network.Key.Public, network.Key, AddressSalt)
	if err != nil {
		return err
	}

	client, ok := t.vpn.Client(network.Key.Public)
	if !ok {
		return ErrNetworkNotFound
	}

	service, err := newDirectoryService(t.logger, client, network.Key)
	if err != nil {
		return err
	}

	api.RegisterDirectoryService(server, service)

	eg, ctx := errgroup.WithContext(ctx)
	ctx, cancel := context.WithCancel(ctx)

	t.servers.Store(network.Id, cancel)
	defer t.servers.Delete(network.Id)

	eg.Go(func() error { return service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	return eg.Wait()
}

func (t *Control) handleNetworkStop(network *pb.Network) {
	if server, ok := t.servers.Load(network.Id); ok {
		server.(context.CancelFunc)()
	}
}

func networkKeyForCertificate(cert *pb.Certificate) []byte {
	return dao.GetRootCert(cert).Key
}
