package chat

import (
	"context"
	"errors"
	"fmt"
	"sync"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

var _ control.ChatControl = &Control{}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer control.DialerControl,
	transfer control.TransferControl,
) *Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		events:    events,
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
	events    chan interface{}
	dialer    control.DialerControl
	transfer  control.TransferControl

	lock    sync.Mutex
	runners llrb.LLRB
}

// Run ...
func (t *Control) Run(ctx context.Context) {
	if err := t.startServerRunners(ctx); err != nil {
		t.logger.Debug("starting chat server runners failed", zap.Error(err))
	}

	for {
		select {
		case <-t.events:
		case e := <-t.events:
			switch e := e.(type) {
			case event.ChatSyncAssets:
				t.syncAssets(e.ServerID, e.ForceUnifiedUpdate)
			}
		case <-ctx.Done():
			return
		}
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

func (t *Control) SyncAssets(serverID uint64, forceUnifiedUpdate bool) error {
	t.observers.EmitGlobal(event.ChatSyncAssets{
		ServerID:           serverID,
		ForceUnifiedUpdate: forceUnifiedUpdate,
	})
	return nil
}

func (t *Control) syncAssets(serverID uint64, forceUnifiedUpdate bool) {
	server, err := dao.GetChatServer(t.store, serverID)
	if err != nil {
		return
	}

	runner, ok := t.runners.Get(&runner{key: server.Key.Public}).(*runner)
	if !ok {
		return
	}

	runner.SyncServer()
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

// ReadServer ...
func (t *Control) ReadServer(ctx context.Context, networkKey, key []byte) (<-chan *chatv1.ServerEvent, <-chan *chatv1.AssetBundle, error) {
	t.lock.Lock()
	defer t.lock.Unlock()

	runner, ok := t.runners.Get(&runner{key: key}).(*runner)
	if !ok {
		runner = newRunner(ctx, t.logger, t.vpn, t.store, t.dialer, t.transfer, key, networkKey, nil)
		t.runners.ReplaceOrInsert(runner)
	}

	events := make(chan *chatv1.ServerEvent)
	assets := make(chan *chatv1.AssetBundle)

	go func() {
		defer close(events)
		defer close(assets)

		logger := t.logger.With(
			logutil.ByteHex("chat", key),
			logutil.ByteHex("network", networkKey),
		)
		for {
			eg, rctx := errgroup.WithContext(ctx)

			eventReader, assetReader, err := runner.Readers(rctx)
			if err != nil {
				logger.Debug("open chat readers failed", zap.Error(err))
				return
			}

			eg.Go(func() error {
				for {
					e := &chatv1.ServerEvent{}
					err := eventReader.Read(e)
					if err == protoutil.ErrShortRead {
						continue
					} else if err != nil {
						return fmt.Errorf("reading event: %w", err)
					}
					events <- e
				}
			})

			eg.Go(func() error {
				for {
					b := &chatv1.AssetBundle{}
					err := assetReader.Read(b)
					if err != nil {
						return fmt.Errorf("reading asset bundle: %w", err)
					}
					assets <- b
				}
			})

			err = eg.Wait()
			done := ctx.Err() != nil

			logger.Debug(
				"chat reader closed",
				zap.Error(err),
				zap.Bool("done", done),
			)
			if done {
				return
			}
		}
	}()

	return events, assets, nil
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
