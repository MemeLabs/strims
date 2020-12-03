package frontend

import (
	"context"
	"errors"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/api"
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

// ErrMethodNotImplemented ...
var ErrMethodNotImplemented = errors.New("method not implemented")

// VPNFunc ...
type VPNFunc func(key *pb.Key) (*vpn.Host, error)

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
	app     *app.Control

	Profile   api.ProfileService
	Network   api.NetworkService
	Debug     api.DebugService
	Bootstrap api.BootstrapService
	Chat      api.ChatService
	Video     api.VideoService
}

func (c *Instance) initProfileService(ctx context.Context, store kv.BlobStore, newVPN VPNFunc) error {
	init := func(profile *pb.Profile, store *dao.ProfileStore) error {
		if err := c.Init(ctx, profile, store); err != nil {
			return err
		}
		c.registerServices()
		return nil
	}

	var err error
	c.Profile, err = newProfileService(ctx, c.logger, store, init)
	if err != nil {
		return err
	}
	api.RegisterProfileService(c.server, c.Profile)

	return nil
}

// Init ...
func (c *Instance) Init(ctx context.Context, profile *pb.Profile, store *dao.ProfileStore) error {
	vpn, err := c.newVPN(profile.Key)
	if err != nil {
		return err
	}

	// TODO: put this somewhere
	// network.NewPeerHandler(c.logger, c.broker, vpn, store, profile)
	c.app = app.NewControl(c.logger, c.broker, vpn, store, profile)
	vpn.VNIC().AddPeerHandler(peer.NewPeerHandler(c.logger, c.app))

	c.Network = newNetworkService(ctx, c.logger, profile, store, vpn, c.app)
	c.Debug = newDebugService(c.logger)

	c.Bootstrap = newBootstrapService(ctx, c.logger, store, vpn, c.app)

	// c.Chat = newChatService(c.logger, store)
	// c.Video = newVideoService(c.logger, store)
	return nil
}

func (c *Instance) registerServices() {
	api.RegisterNetworkService(c.server, c.Network)
	api.RegisterDebugService(c.server, c.Debug)
	api.RegisterBootstrapService(c.server, c.Bootstrap)
	// api.RegisterChatService(c.server, c.Chat)
	// api.RegisterVideoService(c.server, c.Video)
}
