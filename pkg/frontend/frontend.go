package frontend

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"path"
	"runtime"
	"time"

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
	Store         kv.BlobStore
	Logger        *zap.Logger
	NewVPNHost    VPNFunc
	BrokerFactory network.BrokerFactory
}

// Listen ...
func (s *Server) Listen(ctx context.Context, rw io.ReadWriter) error {
	c := New(s.Logger, s.NewVPNHost, s.BrokerFactory)
	if err := c.initProfileService(ctx, s.Logger, s.Store, s.NewVPNHost); err != nil {
		return err
	}

	c.host.Listen(ctx, rw)
	return nil
}

// New ...
func New(logger *zap.Logger, newVPN VPNFunc, brokerFactory network.BrokerFactory) *Instance {
	return &Instance{
		logger:        logger,
		host:          rpc.NewHost(logger),
		newVPN:        newVPN,
		brokerFactory: brokerFactory,
	}
}

// Instance ...
type Instance struct {
	logger        *zap.Logger
	host          *rpc.Host
	newVPN        VPNFunc
	brokerFactory network.BrokerFactory

	profile *pb.Profile
	store   *dao.ProfileStore

	Profile   api.ProfileService
	Network   api.NetworkService
	Debug     api.DebugService
	Bootstrap api.BootstrapService
	Chat      api.ChatService
	Video     api.VideoService
}

func (c *Instance) initProfileService(ctx context.Context, logger *zap.Logger, store kv.BlobStore, newVPN VPNFunc) error {
	init := func(profile *pb.Profile, store *dao.ProfileStore) error {
		if err := c.Init(ctx, profile, store); err != nil {
			return err
		}
		c.registerServices(c.host)
		return nil
	}

	var err error
	c.Profile, err = newProfileService(ctx, logger, store, init)
	if err != nil {
		return err
	}
	api.RegisterProfileService(c.host, c.Profile)

	return nil
}

// Init ...
func (c *Instance) Init(ctx context.Context, profile *pb.Profile, store *dao.ProfileStore) error {
	vpnHost, err := c.newVPN(profile.Key)
	if err != nil {
		return err
	}

	// TODO: put this somewhere
	network.NewPeerHandler(c.logger, c.brokerFactory, vpnHost)

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

func (c *Instance) registerServices(host *rpc.Host) {
	api.RegisterNetworkService(host, c.Network)
	api.RegisterDebugService(host, c.Debug)
	api.RegisterBootstrapService(host, c.Bootstrap)
	api.RegisterChatService(host, c.Chat)
	api.RegisterVideoService(host, c.Video)
}

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}
