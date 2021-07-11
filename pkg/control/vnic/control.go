package vnic

import (
	"context"
	"sync"

	vnicv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vnic/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
) *Control {
	// events := make(chan interface{}, 8)
	// observers.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		// events:        events,
		ingressConfig: &vnicv1.Config{},
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	// events    chan interface{}

	lock          sync.Mutex
	ingressConfig *vnicv1.Config
}

// Run ...
func (c *Control) Run(ctx context.Context) {
	go c.loadConfig()

	// TODO: sync reload?
}

func (c *Control) applyConfig(config *vnicv1.Config) {
	c.vpn.VNIC().QOS().SetRateLimit(config.MaxUploadBytesPerSecond)
}

func (c *Control) loadConfig() {
	config, err := dao.GetVNICConfig(c.store)
	if err != nil {
		c.logger.Debug("failed to load vnic config", zap.Error(err))
		return
	}

	c.applyConfig(config)
}

// GetConfig ...
func (c *Control) GetConfig() (*vnicv1.Config, error) {
	return dao.GetVNICConfig(c.store)
}

// SetConfig ...
func (c *Control) SetConfig(config *vnicv1.Config) error {
	if err := dao.SetVNICConfig(c.store, config); err != nil {
		return err
	}

	c.applyConfig(config)
	return nil
}
