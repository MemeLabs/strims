//go:build !js

package videoingress

import (
	"context"
	"errors"
	"net"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/hashmap"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/sortutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

type server interface {
	Listen() error
	Close() error
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	profile *profilev1.Profile,
	transfer transfer.Control,
	network network.Control,
	directory directory.Control,
) Control {
	return &control{
		ctx:     ctx,
		logger:  logger,
		vpn:     vpn,
		store:   store,
		profile: profile,

		events:                observers.Chan(),
		ingressConfig:         &videov1.VideoIngressConfig{},
		network:               network,
		shareServerCloseFuncs: hashmap.New[[]byte, context.CancelFunc](hashmap.NewByteInterface[[]byte]()),
		ingressService: newIngressService(
			ctx,
			logger,
			store,
			transfer,
			network,
			directory,
		),
	}
}

// Control ...
type control struct {
	ctx     context.Context
	logger  *zap.Logger
	vpn     *vpn.Host
	store   *dao.ProfileStore
	profile *profilev1.Profile
	network network.Control

	events                chan any
	ingressService        *ingressService
	lock                  sync.Mutex
	ingressConfig         *videov1.VideoIngressConfig
	shareServerCloseFuncs hashmap.Map[[]byte, context.CancelFunc]
	ingressServer         server
}

// Run ...
func (c *control) Run() {
	go c.loadIngressConfig()

	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			case event.NetworkStart:
				c.handleNetworkStart(e.Network)
			case event.NetworkStop:
				c.handleNetworkStop(e.Network)
			case *videov1.VideoIngressConfigChangeEvent:
				c.reinitIngress(e.IngressConfig)
			case *videov1.VideoChannelChangeEvent:
				c.handleChannelChange(e.VideoChannel)
			case *videov1.VideoChannelDeleteEvent:
				c.handleChannelDelete(e.VideoChannel.Id)
			}
		case <-c.ctx.Done():
			c.stopIngressServer()
			return
		}
	}
}

func (c *control) handleNetworkStart(network *networkv1.Network) {
	c.lock.Lock()
	defer c.lock.Unlock()

	networkKey := dao.NetworkKey(network)
	if c.ingressConfig.Enabled && sortutil.SearchBytes(c.ingressConfig.ServiceNetworkKeys, networkKey) != -1 {
		c.tryStartIngressShareServer(networkKey)
	}
}

func (c *control) handleNetworkStop(network *networkv1.Network) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.tryStopIngressShareServer(dao.NetworkKey(network))
}

func (c *control) handleChannelChange(channel *videov1.VideoChannel) {
	c.ingressService.UpdateChannel(channel)
}

func (c *control) handleChannelDelete(id uint64) {
	c.ingressService.RemoveChannel(id)
}

func (c *control) loadIngressConfig() {
	config, err := dao.VideoIngressConfig.Get(c.store)
	if err != nil {
		c.logger.Fatal("loading video ingress config failed", zap.Error(err))
	}
	c.reinitIngress(config)
}

func (c *control) reinitIngress(next *videov1.VideoIngressConfig) {
	c.lock.Lock()
	defer c.lock.Unlock()

	prev := c.ingressConfig
	c.ingressConfig = next

	shutdown := prev.Enabled && !next.Enabled
	startup := !prev.Enabled && next.Enabled

	sortutil.Bytes(next.ServiceNetworkKeys)
	var removedNetworkKeys, addedNetworkKeys [][]byte
	if shutdown {
		removedNetworkKeys = prev.ServiceNetworkKeys
	} else if startup {
		addedNetworkKeys = next.ServiceNetworkKeys
	} else if next.Enabled {
		removedNetworkKeys, addedNetworkKeys = sortutil.DiffBytes(prev.ServiceNetworkKeys, next.ServiceNetworkKeys)
	}
	for _, k := range removedNetworkKeys {
		c.tryStopIngressShareServer(k)
	}
	for _, k := range addedNetworkKeys {
		c.tryStartIngressShareServer(k)
	}

	if shutdown {
		c.stopIngressServer()
	} else if startup {
		c.startIngressServer()
	} else if next.Enabled && prev.ServerAddr != next.ServerAddr {
		c.stopIngressServer()
		c.startIngressServer()
	}
}

func (c *control) tryStopIngressShareServer(networkKey []byte) {
	if close, ok := c.shareServerCloseFuncs.Delete(networkKey); ok {
		close()
	}
}

func (c *control) tryStartIngressShareServer(networkKey []byte) {
	ctx, cancel := context.WithCancel(c.ctx)
	c.shareServerCloseFuncs.Set(networkKey, cancel)

	go func() {
		c.logger.Info(
			"starting ingress sharing service",
			logutil.ByteHex("network", networkKey),
		)

		if err := c.startIngressShareServer(ctx, networkKey); err != nil {
			c.logger.Info(
				"ingress sharing service closed",
				logutil.ByteHex("network", networkKey),
				zap.Error(err),
			)
		}
	}()
}

func (c *control) startIngressShareServer(ctx context.Context, networkKey []byte) error {
	server, err := c.network.Dialer().Server(ctx, networkKey, c.profile.Key, ShareAddressSalt)
	if err != nil {
		return err
	}

	node, ok := c.vpn.Node(networkKey)
	if !ok {
		return errors.New("network not found")
	}

	service := newShareService(c.logger, node, c.store)
	if err != nil {
		return err
	}

	videov1.RegisterVideoIngressShareService(server, service)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	return eg.Wait()
}

func (c *control) stopIngressServer() {
	if c.ingressServer != nil {
		if err := c.ingressServer.Close(); err != nil {
			c.logger.Debug("ingress server returned errors while closing", zap.Error(err))
		}
		c.ingressServer = nil
	}
}

func (c *control) createServer() server {
	return &rtmpingress.Server{
		Addr:         c.ingressConfig.ServerAddr,
		HandleStream: c.ingressService.HandleStream,
		BaseContext:  func(net.Conn) context.Context { return c.ctx },
		Logger:       c.logger,
	}
}

func (c *control) createPassthruServer() server {
	return &rtmpingress.PassthruServer{
		Addr:         c.ingressConfig.ServerAddr,
		HandleStream: c.ingressService.HandlePassthruStream,
		BaseContext:  func(net.Conn) context.Context { return c.ctx },
		Logger:       c.logger,
	}
}

func (c *control) startIngressServer() {
	// TODO: allow configuring server mode?
	c.ingressServer = c.createPassthruServer()
	// c.ingressServer = c.createServer()

	go func() {
		c.logger.Debug(
			"starting ingress server",
			zap.String("address", c.ingressConfig.ServerAddr),
		)
		err := c.ingressServer.Listen()
		c.logger.Debug("ingress server closed", zap.Error(err))
	}()
}
