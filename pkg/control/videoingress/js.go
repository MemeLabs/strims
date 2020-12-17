// +build js

package videoingress

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, profile *pb.Profile, observers *event.Observers, transfer *transfer.Control, dialer *dialer.Control, network *network.Control) *Control {
	return &Control{}
}

// Control ...
type Control struct{}

// Run ...
func (c *Control) Run(ctx context.Context) {}
