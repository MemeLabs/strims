package app

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/control/ca"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/swarm"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, broker network.Broker, vpn *vpn.Host, store *dao.ProfileStore, profile *pb.Profile) *Control {
	observers := &event.Observers{}

	var (
		caControl        = ca.NewControl(logger, vpn, store, profile, observers)
		networkControl   = network.NewControl(logger, broker, vpn, store, profile, observers, caControl)
		swarmControl     = swarm.NewControl(logger, vpn, observers)
		bootstrapControl = bootstrap.NewControl(logger, vpn, store, profile, observers)
		peerControl      = NewPeerControl(logger, observers, caControl, networkControl, swarmControl, bootstrapControl)
	)

	c := &Control{
		logger:    logger,
		ca:        caControl,
		peer:      peerControl,
		network:   networkControl,
		swarm:     swarmControl,
		bootstrap: bootstrapControl,
	}

	ctx := context.Background()
	// go c.peer.Run(ctx)
	go c.network.Run(ctx)
	go c.swarm.Run(ctx)
	go c.bootstrap.Run(ctx)

	return c
}

// Control ...
type Control struct {
	logger    *zap.Logger
	ca        *ca.Control
	peer      *PeerControl
	network   *network.Control
	swarm     *swarm.Control
	bootstrap *bootstrap.Control
}

// CA ...
func (c *Control) CA() *ca.Control { return c.ca }

// Peer ...
func (c *Control) Peer() *PeerControl { return c.peer }

// Network ...
func (c *Control) Network() *network.Control { return c.network }

// Swarm ...
func (c *Control) Swarm() *swarm.Control { return c.swarm }

// Bootstrap ...
func (c *Control) Bootstrap() *bootstrap.Control { return c.bootstrap }
