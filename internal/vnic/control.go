package vnic

import (
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	vnicv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vnic/v1"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

type Control interface {
	Run()
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
) Control {
	return &control{
		ctx:    ctx,
		logger: logger,
		vpn:    vpn,
		store:  store,

		events:        observers.Chan(),
		ingressConfig: &vnicv1.Config{},
	}
}

// Control ...
type control struct {
	ctx    context.Context
	logger *zap.Logger
	vpn    *vpn.Host
	store  *dao.ProfileStore

	events        chan any
	lock          sync.Mutex
	ingressConfig *vnicv1.Config
}

// Run ...
func (c *control) Run() {
	go c.loadConfig()

	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			case *vnicv1.ConfigChangeEvent:
				c.applyConfig(e.Config)
			}
		case <-c.ctx.Done():
			return
		}
	}
}

func (c *control) applyConfig(config *vnicv1.Config) {
	c.vpn.VNIC().QOS().SetRateLimit(config.MaxUploadBytesPerSecond)
	c.vpn.VNIC().SetMaxPeers(int(config.MaxPeers))
}

func (c *control) loadConfig() {
	config, err := dao.VNICConfig.Get(c.store)
	if err != nil {
		c.logger.Debug("failed to load vnic config", zap.Error(err))
		return
	}

	c.applyConfig(config)
}
