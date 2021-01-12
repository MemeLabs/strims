package app

import (
	"context"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
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
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, broker network.Broker, vpn *vpn.Host, store *dao.ProfileStore, profile *profilev1.Profile) control.AppControl {
	observers := &event.Observers{}

	var (
		dialerControl       = dialer.NewControl(logger, vpn, profile)
		transferControl     = transfer.NewControl(logger, vpn, observers)
		caControl           = ca.NewControl(logger, vpn, store, observers, dialerControl)
		directoryControl    = directory.NewControl(logger, vpn, store, observers, dialerControl, transferControl)
		networkControl      = network.NewControl(logger, broker, vpn, store, profile, observers, dialerControl)
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
func (c *Control) Dialer() control.DialerControl { return c.dialer }

// CA ...
func (c *Control) CA() control.CAControl { return c.ca }

// Directory ...
func (c *Control) Directory() control.DirectoryControl { return c.directory }

// Peer ...
func (c *Control) Peer() control.PeerControl { return c.peer }

// Network ...
func (c *Control) Network() control.NetworkControl { return c.network }

// Transfer ...
func (c *Control) Transfer() control.TransferControl { return c.transfer }

// Bootstrap ...
func (c *Control) Bootstrap() control.BootstrapControl { return c.bootstrap }

// VideoIngress ...
func (c *Control) VideoIngress() control.VideoIngressControl { return c.videoingress }

// VideoChannel ...
func (c *Control) VideoChannel() control.VideoChannelControl { return c.videochannel }
