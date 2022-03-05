//go:build js

package videoingress

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

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
	return &control{}
}

// Control ...
type control struct{}

// Run ...
func (c *control) Run() {}
