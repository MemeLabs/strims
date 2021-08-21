package frontend

import (
	"context"
	"io"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/services/peer"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

// use a higher limit here to prevent errors while streaming video from the ui.
const serverMaxMessageBytes = 10 * 1024 * 1024

// VPNFunc ...
type VPNFunc func(key *key.Key) (*vpn.Host, error)

// ServiceFunc ...
type ServiceFunc func(server *rpc.Server, params *ServiceParams)

var services []ServiceFunc

// RegisterService ...
func RegisterService(fn ServiceFunc) {
	services = append(services, fn)
}

// Server ...
type Server struct {
	Store      kv.BlobStore
	Logger     *zap.Logger
	NewVPNHost VPNFunc
	Broker     network.Broker
}

// Listen ...
func (s *Server) Listen(ctx context.Context, rw io.ReadWriter) error {
	server := rpc.NewServer(s.Logger, &rpc.RWDialer{
		Logger:          s.Logger,
		ReadWriter:      rw,
		MaxMessageBytes: serverMaxMessageBytes,
	})
	c := New(s.Logger, server, s.NewVPNHost, s.Broker)
	if err := c.initProfileService(ctx, s.Store); err != nil {
		return err
	}

	return server.Listen(ctx)
}

// New ...
func New(logger *zap.Logger, server *rpc.Server, newVPN VPNFunc, broker network.Broker) *Instance {
	return &Instance{
		logger: logger,
		server: server,
		newVPN: newVPN,
		broker: broker,
	}
}

// Instance ...
type Instance struct {
	logger *zap.Logger
	server *rpc.Server
	newVPN VPNFunc
	broker network.Broker
}

func (c *Instance) initProfileService(ctx context.Context, store kv.BlobStore) error {
	init := func(profile *profilev1.Profile, store *dao.ProfileStore) error {
		if err := c.Init(ctx, profile, store); err != nil {
			return err
		}
		return nil
	}

	profileService, err := newProfileService(c.logger, store, init)
	if err != nil {
		return err
	}
	profilev1.RegisterProfileServiceService(c.server, profileService)

	return nil
}

// Init ...
func (c *Instance) Init(ctx context.Context, profile *profilev1.Profile, store *dao.ProfileStore) error {
	vpn, err := c.newVPN(profile.Key)
	if err != nil {
		return err
	}

	app := app.NewControl(c.logger, c.broker, vpn, store, profile)
	go app.Run(ctx)

	qosc := vpn.VNIC().QOS().AddClass(1)
	vpn.VNIC().AddPeerHandler(peer.NewPeerHandler(c.logger, app, store, qosc))

	for _, fn := range services {
		fn(c.server, &ServiceParams{
			Context: ctx,
			Logger:  c.logger,
			Profile: profile,
			Store:   store,
			VPN:     vpn,
			App:     app,
		})
	}

	return nil
}

// ServiceParams ...
type ServiceParams struct {
	Context context.Context
	Logger  *zap.Logger
	Profile *profilev1.Profile
	Store   *dao.ProfileStore
	VPN     *vpn.Host
	App     control.AppControl
}
