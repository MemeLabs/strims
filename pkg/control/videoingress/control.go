// +build !js

package videoingress

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/sortutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, profile *pb.Profile, observers *event.Observers, transfer *transfer.Control, dialer *dialer.Control, network *network.Control) *Control {
	events := make(chan interface{}, 128)
	observers.Notify(events)

	return &Control{
		logger:         logger,
		vpn:            vpn,
		store:          store,
		profile:        profile,
		observers:      observers,
		events:         events,
		ingressConfig:  &pb.VideoIngressConfig{},
		dialer:         dialer,
		ingressService: newIngressService(logger, store, transfer, dialer, network),
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	profile   *pb.Profile
	observers *event.Observers
	events    chan interface{}
	dialer    *dialer.Control

	ingressService *ingressService
	lock           sync.Mutex
	ingressConfig  *pb.VideoIngressConfig
	shareServers   llrb.LLRB
	ingressServer  *rtmpingress.Server
}

// Run ...
func (c *Control) Run(ctx context.Context) {
	go c.loadIngressConfig()

	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			case event.NetworkStart:
				c.handleNetworkStart(ctx, e.Network)
			case event.NetworkStop:
				c.handleNetworkStop(e.Network)
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

func (c *Control) handleNetworkStart(ctx context.Context, network *pb.Network) {
	c.lock.Lock()
	defer c.lock.Unlock()

	networkKey := dao.NetworkKey(network)
	if c.ingressConfig.Enabled && sortutil.SearchBytes(c.ingressConfig.ServiceNetworkKeys, networkKey) != -1 {
		c.tryStartIngressShareServer(networkKey)
	}
}

func (c *Control) handleNetworkStop(network *pb.Network) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.tryStopIngressShareServer(dao.NetworkKey(network))
}

func (c *Control) handleChannelUpdate(channel *pb.VideoChannel) {
	c.ingressService.UpdateChannel(channel)
}

func (c *Control) handleChannelRemove(id uint64) {
	c.ingressService.RemoveChannel(id)
}

func (c *Control) loadIngressConfig() {
	config, err := dao.GetVideoIngressConfig(c.store)
	if err != nil {
		c.logger.Fatal("loading video ingress config failed", zap.Error(err))
	}
	c.reinitIngress(config)
}

func (c *Control) reinitIngress(next *pb.VideoIngressConfig) {
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

func (c *Control) tryStopIngressShareServer(networkKey []byte) {
	if it := c.shareServers.Delete(&videoIngressServersItem{networkKey: networkKey}); it != nil {
		it.(*videoIngressServersItem).close()
	}
}

func (c *Control) tryStartIngressShareServer(networkKey []byte) {
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

func (c *Control) startIngressShareServer(ctx context.Context, networkKey []byte) error {
	server, err := c.dialer.Server(networkKey, c.profile.Key, ShareAddressSalt)
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

	api.RegisterVideoIngressShareService(server, service)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	return eg.Wait()
}

func (c *Control) stopIngressServer() {
	if c.ingressServer != nil {
		if err := c.ingressServer.Close(); err != nil {
			c.logger.Debug("ingress server returned errors while closing", zap.Error(err))
		}
		c.ingressServer = nil
	}
}

func (c *Control) startIngressServer() {
	c.ingressServer = &rtmpingress.Server{
		Addr:         c.ingressConfig.ServerAddr,
		HandleStream: c.ingressService.HandleStream,
	}

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
func (c *Control) GetIngressConfig() (*pb.VideoIngressConfig, error) {
	return dao.GetVideoIngressConfig(c.store)
}

// SetIngressConfig ...
func (c *Control) SetIngressConfig(config *pb.VideoIngressConfig) error {
	// TODO: change stuff...
	if err := dao.SetVideoIngressConfig(c.store, config); err != nil {
		return err
	}

	c.reinitIngress(config)
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
