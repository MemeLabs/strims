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
	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
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
	if err := t.startServerRunners(ctx); err != nil {
		t.logger.Debug("starting chat server runners failed", zap.Error(err))
	}
}

func (t *Control) startServerRunners(ctx context.Context) error {
	configs, err := dao.GetChatServers(t.store)
	if err != nil {
		return err
	}

	for _, config := range configs {
		t.runners.ReplaceOrInsert(newRunner(ctx, t.logger, t.vpn, t.store, t.dialer, t.transfer, config.Key.Public, config.NetworkKey, config))
	}
	return nil
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

func (t *Control) client(networkKey, key []byte) (*rpc.Client, *chatv1.ChatClient, error) {
	client, err := t.dialer.Client(networkKey, key, ServiceAddressSalt)
	if err != nil {
		return nil, nil, err
	}

	return client, chatv1.NewChatClient(client), nil
}

// ReadServerEvents ...
func (t *Control) ReadServerEvents(ctx context.Context, networkKey, key []byte) (<-chan *chatv1.ServerEvent, error) {
	t.lock.Lock()
	defer t.lock.Unlock()

	runner, ok := t.runners.Get(&runner{key: key}).(*runner)
	if !ok {
		runner = newRunner(ctx, t.logger, t.vpn, t.store, t.dialer, t.transfer, key, networkKey, nil)
		t.runners.ReplaceOrInsert(runner)
	}

	ch := make(chan *chatv1.ServerEvent)

	go func() {
		logger := t.logger.With(
			logutil.ByteHex("chat", key),
			logutil.ByteHex("network", networkKey),
		)
		for {
			er, err := runner.EventReader(ctx)
			if err != nil {
				close(ch)
				logger.Debug("chat event reader closed", zap.Error(err))
			}

			for {
				e := &chatv1.ServerEvent{}
				err := er.Read(e)
				if err == protoutil.ErrShortRead {
					continue
				} else if err != nil {
					logger.Debug("error reading chat event", zap.Error(err))
					break
				}

				debug.PrintJSON(e)

				ch <- e
			}
		}
	}()

	return ch, nil
}

// SendMessage ...
func (t *Control) SendMessage(ctx context.Context, networkKey, key []byte, m string) error {
	c, cc, err := t.client(networkKey, key)
	if err != nil {
		return err
	}
	defer c.Close()

	return cc.SendMessage(ctx, &chatv1.SendMessageRequest{Body: m}, &chatv1.SendMessageResponse{})
}
