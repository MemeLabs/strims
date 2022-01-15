//go:build !js
// +build !js

package videoingress

import (
	"bytes"
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
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/sortutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

type server interface {
	Listen() error
	Close() error
}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	profile *profilev1.Profile,
	observers *event.Observers,
	transfer transfer.Control,
	network network.Control,
	directory directory.Control,
) Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	return &control{
		logger:        logger,
		vpn:           vpn,
		store:         store,
		profile:       profile,
		observers:     observers,
		events:        events,
		ingressConfig: &videov1.VideoIngressConfig{},
		network:       network,
		ingressService: newIngressService(
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
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	profile   *profilev1.Profile
	observers *event.Observers
	events    chan interface{}
	network   network.Control

	ingressService *ingressService
	lock           sync.Mutex
	ingressConfig  *videov1.VideoIngressConfig
	shareServers   llrb.LLRB
	ingressServer  server
}

// Run ...
func (c *control) Run(ctx context.Context) {
	go c.loadIngressConfig(ctx)

	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			case event.NetworkStart:
				c.handleNetworkStart(ctx, e.Network)
			case event.NetworkStop:
				c.handleNetworkStop(e.Network)
			case event.VideoIngressConfigUpdate:
				c.reinitIngress(ctx, e.Config)
			case event.VideoChannelUpdate:
				c.handleChannelUpdate(e.Channel)
			case event.VideoChannelRemove:
				c.handleChannelRemove(e.ID)
			}
		case <-ctx.Done():
			c.stopIngressServer()
			return
		}
	}
}

func (c *control) handleNetworkStart(ctx context.Context, network *networkv1.Network) {
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

func (c *control) handleChannelUpdate(channel *videov1.VideoChannel) {
	c.ingressService.UpdateChannel(channel)
}

func (c *control) handleChannelRemove(id uint64) {
	c.ingressService.RemoveChannel(id)
}

func (c *control) loadIngressConfig(ctx context.Context) {
	config, err := dao.GetVideoIngressConfig(c.store)
	if err != nil {
		c.logger.Fatal("loading video ingress config failed", zap.Error(err))
	}
	c.reinitIngress(ctx, config)
}

func (c *control) reinitIngress(ctx context.Context, next *videov1.VideoIngressConfig) {
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
		c.startIngressServer(ctx)
	} else if next.Enabled && prev.ServerAddr != next.ServerAddr {
		c.stopIngressServer()
		c.startIngressServer(ctx)
	}
}

func (c *control) tryStopIngressShareServer(networkKey []byte) {
	if it := c.shareServers.Delete(&videoIngressServersItem{networkKey: networkKey}); it != nil {
		it.(*videoIngressServersItem).close()
	}
}

func (c *control) tryStartIngressShareServer(networkKey []byte) {
	ctx, cancel := context.WithCancel(context.Background())
	c.shareServers.InsertNoReplace(&videoIngressServersItem{networkKey, cancel})

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

func (c *control) createServer(ctx context.Context) server {
	return &rtmpingress.Server{
		Addr:         c.ingressConfig.ServerAddr,
		HandleStream: c.ingressService.HandleStream,
		BaseContext:  func(net.Conn) context.Context { return ctx },
		Logger:       c.logger,
	}
}

func (c *control) createPassthruServer(ctx context.Context) server {
	return &rtmpingress.PassthruServer{
		Addr:         c.ingressConfig.ServerAddr,
		HandleStream: c.ingressService.HandlePassthruStream,
		BaseContext:  func(net.Conn) context.Context { return ctx },
		Logger:       c.logger,
	}
}

func (c *control) startIngressServer(ctx context.Context) {
	// TODO: allow configuring server mode?
	c.ingressServer = c.createPassthruServer(ctx)
	// c.ingressServer = c.createServer(ctx)

	go func() {
		c.logger.Debug(
			"starting ingress server",
			zap.String("address", c.ingressConfig.ServerAddr),
		)
		err := c.ingressServer.Listen()
		c.logger.Debug("ingress server closed", zap.Error(err))
	}()
}

// GetIngressConfig ...
func (c *control) GetIngressConfig() (*videov1.VideoIngressConfig, error) {
	return dao.GetVideoIngressConfig(c.store)
}

// SetIngressConfig ...
func (c *control) SetIngressConfig(config *videov1.VideoIngressConfig) error {
	if err := dao.SetVideoIngressConfig(c.store, config); err != nil {
		return err
	}

	c.observers.EmitGlobal(event.VideoIngressConfigUpdate{Config: config})
	return nil
}

type videoIngressServersItem struct {
	networkKey []byte
	close      context.CancelFunc
}

func (e *videoIngressServersItem) Less(o llrb.Item) bool {
	if o, ok := o.(*videoIngressServersItem); ok {
		return bytes.Compare(e.networkKey, o.networkKey) == -1
	}
	return !o.Less(e)
}
