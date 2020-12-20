package app

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/control/ca"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/control/videochannel"
	"github.com/MemeLabs/go-ppspp/pkg/control/videoingress"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, broker network.Broker, vpn *vpn.Host, store *dao.ProfileStore, profile *pb.Profile) *Control {
	observers := &event.Observers{}

	var (
		dialerControl       = dialer.NewControl(logger, vpn, profile)
		caControl           = ca.NewControl(logger, vpn, store, observers, dialerControl)
		directoryControl    = directory.NewControl(logger, vpn, store, observers, dialerControl)
		networkControl      = network.NewControl(logger, broker, vpn, store, profile, observers, dialerControl)
		transferControl     = transfer.NewControl(logger, vpn, observers)
		bootstrapControl    = bootstrap.NewControl(logger, vpn, store, observers)
		videoingressControl = videoingress.NewControl(logger, vpn, store, profile, observers, transferControl, dialerControl, networkControl)
		videochannelControl = videochannel.NewControl(logger, vpn, store, observers)
		peerControl         = NewPeerControl(logger, observers, caControl, networkControl, transferControl, bootstrapControl)
	)

	return &Control{
		logger:       logger,
		dialer:       dialerControl,
		ca:           caControl,
		directory:    directoryControl,
		peer:         peerControl,
		network:      networkControl,
		transfer:     transferControl,
		bootstrap:    bootstrapControl,
		videoingress: videoingressControl,
		videochannel: videochannelControl,
	}
}

// Control ...
type Control struct {
	logger       *zap.Logger
	dialer       *dialer.Control
	ca           *ca.Control
	directory    *directory.Control
	peer         *PeerControl
	network      *network.Control
	transfer     *transfer.Control
	bootstrap    *bootstrap.Control
	videoingress *videoingress.Control
	videochannel *videochannel.Control
}

// Run ...
func (c *Control) Run(ctx context.Context) {
	go c.ca.Run(ctx)
	go c.directory.Run(ctx)
	go c.network.Run(ctx)
	go c.transfer.Run(ctx)
	go c.bootstrap.Run(ctx)
	go c.videoingress.Run(ctx)
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

// Transfer ...
func (c *Control) Transfer() *transfer.Control { return c.transfer }

// Bootstrap ...
func (c *Control) Bootstrap() *bootstrap.Control { return c.bootstrap }

// VideoIngress ...
func (c *Control) VideoIngress() *videoingress.Control { return c.videoingress }

// VideoChannel ...
func (c *Control) VideoChannel() *videochannel.Control { return c.videochannel }
