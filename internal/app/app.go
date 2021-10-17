package app

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/bootstrap"
	"github.com/MemeLabs/go-ppspp/internal/chat"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	"github.com/MemeLabs/go-ppspp/internal/videocapture"
	"github.com/MemeLabs/go-ppspp/internal/videochannel"
	"github.com/MemeLabs/go-ppspp/internal/videoegress"
	"github.com/MemeLabs/go-ppspp/internal/videoingress"
	"github.com/MemeLabs/go-ppspp/internal/vnic"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// Control ...
type Control interface {
	Run(ctx context.Context)
	Events() *event.Observers
	Peer() PeerControl
	Bootstrap() bootstrap.Control
	Chat() chat.Control
	Directory() directory.Control
	Network() network.Control
	Transfer() transfer.Control
	VideoCapture() videocapture.Control
	VideoChannel() videochannel.Control
	VideoEgress() videoegress.Control
	VideoIngress() videoingress.Control
	VNIC() vnic.Control
}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	broker network.Broker,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	profile *profilev1.Profile,
) Control {
	observers := &event.Observers{}

	var (
		transferControl     = transfer.NewControl(logger, vpn, observers)
		networkControl      = network.NewControl(logger, broker, vpn, store, profile, observers)
		chatControl         = chat.NewControl(logger, vpn, store, observers, networkControl, transferControl)
		directoryControl    = directory.NewControl(logger, vpn, store, observers, networkControl, transferControl)
		bootstrapControl    = bootstrap.NewControl(logger, vpn, store, observers)
		videocaptureControl = videocapture.NewControl(logger, transferControl, directoryControl, networkControl)
		videoingressControl = videoingress.NewControl(logger, vpn, store, profile, observers, transferControl, networkControl, directoryControl)
		videochannelControl = videochannel.NewControl(logger, vpn, store, observers)
		videoegressControl  = videoegress.NewControl(logger, vpn, observers, transferControl)
		vnicControl         = vnic.NewControl(logger, vpn, store, observers)
		peerControl         = NewPeerControl(logger, observers, networkControl, transferControl, bootstrapControl)
	)

	return &control{
		logger:       logger,
		observers:    observers,
		chat:         chatControl,
		directory:    directoryControl,
		peer:         peerControl,
		network:      networkControl,
		transfer:     transferControl,
		bootstrap:    bootstrapControl,
		videocapture: videocaptureControl,
		videoingress: videoingressControl,
		videochannel: videochannelControl,
		videoegress:  videoegressControl,
		vnic:         vnicControl,
	}
}

// Control ...
type control struct {
	logger       *zap.Logger
	observers    *event.Observers
	peer         PeerControl
	bootstrap    bootstrap.Control
	chat         chat.Control
	directory    directory.Control
	network      network.Control
	transfer     transfer.Control
	videocapture videocapture.Control
	videochannel videochannel.Control
	videoegress  videoegress.Control
	videoingress videoingress.Control
	vnic         vnic.Control
}

// Run ...
func (c *control) Run(ctx context.Context) {
	go c.chat.Run(ctx)
	go c.directory.Run(ctx)
	go c.network.Run(ctx)
	go c.transfer.Run(ctx)
	go c.bootstrap.Run(ctx)
	go c.videoingress.Run(ctx)
	go c.vnic.Run(ctx)
}

func (c *control) Events() *event.Observers {
	return c.observers
}

// Peer ...
func (c *control) Peer() PeerControl { return c.peer }

// Directory ...
func (c *control) Directory() directory.Control { return c.directory }

// Chat ...
func (c *control) Chat() chat.Control { return c.chat }

// Network ...
func (c *control) Network() network.Control { return c.network }

// Transfer ...
func (c *control) Transfer() transfer.Control { return c.transfer }

// Bootstrap ...
func (c *control) Bootstrap() bootstrap.Control { return c.bootstrap }

// VideoCapture ...
func (c *control) VideoCapture() videocapture.Control { return c.videocapture }

// VideoChannel ...
func (c *control) VideoChannel() videochannel.Control { return c.videochannel }

// VideoEgress ...
func (c *control) VideoEgress() videoegress.Control { return c.videoegress }

// VideoIngress ...
func (c *control) VideoIngress() videoingress.Control { return c.videoingress }

// VNIC ...
func (c *control) VNIC() vnic.Control { return c.vnic }
