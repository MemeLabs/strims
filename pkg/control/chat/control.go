package chat

import (
	"context"
	"errors"
	"sync"

	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers, dialer *dialer.Control, transfer *transfer.Control) *Control {
	return &Control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		dialer:    dialer,
		transfer:  transfer,
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    *dialer.Control
	transfer  *transfer.Control

	lock    sync.Mutex
	runners llrb.LLRB
}

// Run ...
func (t *Control) Run(ctx context.Context) {

}

// SyncServer ...
func (t *Control) SyncServer(s *chatv1.Server) {
	t.observers.EmitLocal(event.ChatServerSync{Server: s})
}

// RemoveServer ...
func (t *Control) RemoveServer(id uint64) {
	t.observers.EmitLocal(event.ChatServerRemove{ID: id})
}

// SyncEmote ...
func (t *Control) SyncEmote(serverID uint64, e *chatv1.Emote) {
	t.observers.EmitLocal(event.ChatEmoteSync{
		ServerID: serverID,
		Emote:    e,
	})
}

// RemoveEmote ...
func (t *Control) RemoveEmote(id uint64) {
	t.observers.EmitLocal(event.ChatEmoteRemove{ID: id})
}
