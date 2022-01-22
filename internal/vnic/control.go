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
	Run(ctx context.Context)
	GetConfig() (*vnicv1.Config, error)
	SetConfig(config *vnicv1.Config) error
}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
) Control {
	// events := make(chan interface{}, 8)
	// observers.Notify(events)

	return &control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		// events:        events,
		ingressConfig: &vnicv1.Config{},
	}
}

// Control ...
type control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	// events    chan interface{}

	lock          sync.Mutex
	ingressConfig *vnicv1.Config
}

// Run ...
func (c *control) Run(ctx context.Context) {
	go c.loadConfig()

	// TODO: sync reload?
}

func (c *control) applyConfig(config *vnicv1.Config) {
	c.vpn.VNIC().QOS().SetRateLimit(config.MaxUploadBytesPerSecond)
}

func (c *control) loadConfig() {
	config, err := dao.VNICConfig.Get(c.store)
	if err != nil {
		c.logger.Debug("failed to load vnic config", zap.Error(err))
		return
	}

	c.applyConfig(config)
}

// GetConfig ...
func (c *control) GetConfig() (*vnicv1.Config, error) {
	return dao.VNICConfig.Get(c.store)
}

// SetConfig ...
func (c *control) SetConfig(config *vnicv1.Config) error {
	if err := dao.VNICConfig.Set(c.store, config); err != nil {
		return err
	}

	c.applyConfig(config)
	return nil
}
