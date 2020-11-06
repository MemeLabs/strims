package frontend

import (
	"context"
	"errors"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/services/network"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

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
	server := rpc.NewServer(s.Logger)
	c := New(s.Logger, server, s.NewVPNHost, s.Broker)
	if err := c.initProfileService(ctx, s.Store, s.NewVPNHost); err != nil {
		return err
	}

	return server.Listen(ctx, &rpc.RWDialer{
		Logger:     s.Logger,
		ReadWriter: rw,
	})
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
	vpnHost, err := c.newVPN(profile.Key)
	if err != nil {
		return err
	}

	// TODO: put this somewhere
	network.NewPeerHandler(c.logger, c.broker, vpnHost)

	c.Network, err = newNetworkService(ctx, c.logger, profile, store, vpnHost)
	if err != nil {
		return err
	}

	c.Debug = newDebugService(c.logger)

	c.Bootstrap, err = newBootstrapService(ctx, c.logger, store, vpnHost)
	if err != nil {
		return err
	}

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
