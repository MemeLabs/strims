package app

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/control/ca"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
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
		dialerControl    = dialer.NewControl(logger, vpn, profile)
		caControl        = ca.NewControl(logger, vpn, store, observers, dialerControl)
		directoryControl = directory.NewControl(logger, vpn, store, observers)
		networkControl   = network.NewControl(logger, broker, vpn, store, profile, observers, dialerControl)
		swarmControl     = swarm.NewControl(logger, vpn, observers)
		bootstrapControl = bootstrap.NewControl(logger, vpn, store, observers)
		peerControl      = NewPeerControl(logger, observers, caControl, networkControl, swarmControl, bootstrapControl)
	)

	c := &Control{
		logger:    logger,
		dialer:    dialerControl,
		ca:        caControl,
		directory: directoryControl,
		peer:      peerControl,
		network:   networkControl,
		swarm:     swarmControl,
		bootstrap: bootstrapControl,
	}

	ctx := context.Background()
	go c.ca.Run(ctx)
	go c.network.Run(ctx)
	go c.swarm.Run(ctx)
	go c.bootstrap.Run(ctx)

	return c
}

// Control ...
type Control struct {
	logger    *zap.Logger
	dialer    *dialer.Control
	ca        *ca.Control
	directory *directory.Control
	peer      *PeerControl
	network   *network.Control
	swarm     *swarm.Control
	bootstrap *bootstrap.Control
}

// Dialer ...
func (c *Control) Dialer() *dialer.Control { return c.dialer }

// CA ...
func (c *Control) CA() *ca.Control { return c.ca }

// Directory ...
func (c *Control) Directory() *directory.Control { return c.directory }

// Peer ...
func (c *Control) Peer() *PeerControl { return c.peer }

// Network ...
func (c *Control) Network() *network.Control { return c.network }

// Swarm ...
func (c *Control) Swarm() *swarm.Control { return c.swarm }

// Bootstrap ...
func (c *Control) Bootstrap() *bootstrap.Control { return c.bootstrap }
