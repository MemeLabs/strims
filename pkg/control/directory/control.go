package directory

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
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers) *Control {
	events := make(chan interface{}, 128)
	observers.Network.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		observers: observers,
		events:    events,
		// servers:   map[uint64]*Server{},
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	observers *event.Observers
	lock      sync.Mutex
	// servers           map[uint64]*Server
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
	// if network.Key == nil {
	// 	return
	// }

	// client, ok := t.vpn.Client(network.Key.Public)
	// if !ok {
	// 	return
	// }

	// t.logger.Info(
	// 	"starting certificate authority",
	// 	logutil.ByteHex("network", network.Key.Public),
	// )
	// ca, err := NewServer(ctx, t.logger, client, network)
	// if err != nil {
	// 	t.logger.Error(
	// 		"starting certificate authority failed",
	// 		logutil.ByteHex("network", network.Key.Public),
	// 		zap.Error(err),
	// 	)
	// }

	// t.servers[network.Id] = ca
}

func (t *Control) handleNetworkStop(network *pb.Network) {
	// if server, ok := t.servers[network.Id]; ok {
	// 	server.Close()
	// }
}

func networkKeyForCertificate(cert *pb.Certificate) []byte {
	return dao.GetRootCert(cert).Key
}
