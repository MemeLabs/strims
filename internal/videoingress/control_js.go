//go:build js
// +build js

package videoingress

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

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
	return &control{}
}

// Control ...
type control struct{}

// Run ...
func (c *control) Run(ctx context.Context) {}

// GetIngressConfig ...
func (c *control) GetIngressConfig() (*videov1.VideoIngressConfig, error) { return nil, nil }

// SetIngressConfig ...
func (c *control) SetIngressConfig(config *videov1.VideoIngressConfig) error { return nil }
