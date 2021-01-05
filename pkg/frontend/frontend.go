package frontend

import (
	"context"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/services/peer"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// use a higher limit here to prevent errors while streaming video from the ui.
const serverMaxMessageBytes = 10 * 1024 * 1024

// VPNFunc ...
type VPNFunc func(key *pb.Key) (*vpn.Host, error)

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
	if err := c.initProfileService(ctx, s.Store, s.NewVPNHost); err != nil {
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

	profile *pb.Profile
	store   *dao.ProfileStore
	app     control.AppControl
}

func (c *Instance) initProfileService(ctx context.Context, store kv.BlobStore, newVPN VPNFunc) error {
	init := func(profile *pb.Profile, store *dao.ProfileStore) error {
		if err := c.Init(ctx, profile, store); err != nil {
			return err
		}
		return nil
	}

	profileService, err := newProfileService(c.logger, store, init)
	if err != nil {
		return err
	}
	api.RegisterProfileService(c.server, profileService)

	return nil
}

// Init ...
func (c *Instance) Init(ctx context.Context, profile *pb.Profile, store *dao.ProfileStore) error {
	vpn, err := c.newVPN(profile.Key)
	if err != nil {
		return err
	}

	c.app = app.NewControl(c.logger, c.broker, vpn, store, profile)
	go c.app.Run(ctx)

	vpn.VNIC().AddPeerHandler(peer.NewPeerHandler(c.logger, c.app, store))

	for _, fn := range services {
		fn(c.server, &ServiceParams{
			Context: ctx,
			Logger:  c.logger,
			Profile: profile,
			Store:   store,
			VPN:     vpn,
			App:     c.app,
		})
	}

	return nil
}

// ServiceParams ...
type ServiceParams struct {
	Context context.Context
	Logger  *zap.Logger
	Profile *pb.Profile
	Store   *dao.ProfileStore
	VPN     *vpn.Host
	App     control.AppControl
}
