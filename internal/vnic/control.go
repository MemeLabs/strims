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
	GetConfig() (*vnicv1.Config, error)
	SetConfig(config *vnicv1.Config) error
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

	events        chan interface{}
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
	return dao.VNICConfig.Set(c.store, config)
}
