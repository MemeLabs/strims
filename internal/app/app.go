package app

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/autoseed"
	"github.com/MemeLabs/go-ppspp/internal/bootstrap"
	"github.com/MemeLabs/go-ppspp/internal/chat"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/notification"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	"github.com/MemeLabs/go-ppspp/internal/videocapture"
	"github.com/MemeLabs/go-ppspp/internal/videochannel"
	"github.com/MemeLabs/go-ppspp/internal/videoegress"
	"github.com/MemeLabs/go-ppspp/internal/videoingress"
	"github.com/MemeLabs/go-ppspp/internal/vnic"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/httputil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// Control ...
type Control interface {
	Events() *event.Observers
	Peer() PeerControl
	Bootstrap() bootstrap.Control
	Chat() chat.Control
	Directory() directory.Control
	Network() network.Control
	Notification() notification.Control
	Transfer() transfer.Control
	VideoCapture() videocapture.Control
	VideoChannel() videochannel.Control
	VideoEgress() videoegress.Control
	VideoIngress() videoingress.Control
	VNIC() vnic.Control
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	httpmux *httputil.MapServeMux,
	broker network.Broker,
	profile *profilev1.Profile,
) Control {
	var (
		notificationControl = notification.NewControl(logger, store, observers)
		transferControl     = transfer.NewControl(ctx, logger, vpn, store, observers)
		networkControl      = network.NewControl(ctx, logger, vpn, store, observers, transferControl, broker, profile, notificationControl)
		directoryControl    = directory.NewControl(ctx, logger, vpn, store, observers, networkControl, transferControl)
		chatControl         = chat.NewControl(ctx, logger, store, observers, profile, networkControl, transferControl, directoryControl)
		bootstrapControl    = bootstrap.NewControl(ctx, logger, vpn, store, observers)
		videocaptureControl = videocapture.NewControl(ctx, logger, transferControl, directoryControl, networkControl)
		videoingressControl = videoingress.NewControl(ctx, logger, vpn, store, observers, profile, transferControl, networkControl, directoryControl)
		videochannelControl = videochannel.NewControl(store)
		videoegressControl  = videoegress.NewControl(ctx, logger, store, observers, httpmux, profile, transferControl)
		vnicControl         = vnic.NewControl(ctx, logger, vpn, store, observers)
		autoseedControl     = autoseed.NewControl(ctx, logger, store, observers, transferControl)
		peerControl         = NewPeerControl(observers, networkControl, transferControl, bootstrapControl)
	)

	go chatControl.Run()
	go directoryControl.Run()
	go networkControl.Run()
	go transferControl.Run()
	go bootstrapControl.Run()
	go videoingressControl.Run()
	go videoegressControl.Run()
	go autoseedControl.Run()
	go vnicControl.Run()

	return &control{
		observers:    observers,
		chat:         chatControl,
		directory:    directoryControl,
		peer:         peerControl,
		network:      networkControl,
		notification: notificationControl,
		transfer:     transferControl,
		bootstrap:    bootstrapControl,
		videocapture: videocaptureControl,
		videoingress: videoingressControl,
		videochannel: videochannelControl,
		videoegress:  videoegressControl,
		autoseed:     autoseedControl,
		vnic:         vnicControl,
	}
}

// Control ...
type control struct {
	observers    *event.Observers
	peer         PeerControl
	bootstrap    bootstrap.Control
	chat         chat.Control
	directory    directory.Control
	network      network.Control
	notification notification.Control
	transfer     transfer.Control
	videocapture videocapture.Control
	videochannel videochannel.Control
	videoegress  videoegress.Control
	videoingress videoingress.Control
	autoseed     autoseed.Control
	vnic         vnic.Control
}

func (c *control) Events() *event.Observers           { return c.observers }
func (c *control) Peer() PeerControl                  { return c.peer }
func (c *control) Directory() directory.Control       { return c.directory }
func (c *control) Chat() chat.Control                 { return c.chat }
func (c *control) Network() network.Control           { return c.network }
func (c *control) Notification() notification.Control { return c.notification }
func (c *control) Transfer() transfer.Control         { return c.transfer }
func (c *control) Bootstrap() bootstrap.Control       { return c.bootstrap }
func (c *control) VideoCapture() videocapture.Control { return c.videocapture }
func (c *control) VideoChannel() videochannel.Control { return c.videochannel }
func (c *control) VideoEgress() videoegress.Control   { return c.videoegress }
func (c *control) VideoIngress() videoingress.Control { return c.videoingress }
func (c *control) VNIC() vnic.Control                 { return c.vnic }
