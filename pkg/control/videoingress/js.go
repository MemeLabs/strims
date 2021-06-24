//go:build js

package videoingress

import (
	"context"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
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
	transfer *transfer.Control,
	dialer *dialer.Control,
	network *network.Control,
	directory *directory.Control,
) *Control {
	return &Control{}
}

// Control ...
type Control struct{}

// Run ...
func (c *Control) Run(ctx context.Context) {}

// GetIngressConfig ...
func (c *Control) GetIngressConfig() (*videov1.VideoIngressConfig, error) { return nil, nil }

// SetIngressConfig ...
func (c *Control) SetIngressConfig(config *videov1.VideoIngressConfig) error { return nil }
