package chat

import (
	"context"
	"errors"
	"fmt"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

type Control interface {
	Run(ctx context.Context)
	SyncAssets(serverID uint64, forceUnifiedUpdate bool) error
	ReadServer(ctx context.Context, networkKey, key []byte) (<-chan *chatv1.ServerEvent, <-chan *chatv1.AssetBundle, error)
	SendMessage(ctx context.Context, networkKey, key []byte, m string) error
}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	network network.Control,
	transfer transfer.Control,
	directory directory.Control,
) Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	return &control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		events:    events,
		network:   network,
		transfer:  transfer,
		directory: directory,
	}
}

// Control ...
type control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	events    chan interface{}
	network   network.Control
	transfer  transfer.Control
	directory directory.Control

	lock    sync.Mutex
	runners llrb.LLRB
}

// Run ...
func (t *control) Run(ctx context.Context) {
	if err := t.startServerRunners(ctx); err != nil {
		t.logger.Debug("starting chat server runners failed", zap.Error(err))
	}

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case *chatv1.ServerChangeEvent:
				t.handleServerChange(ctx, e.Server)
			case *chatv1.ServerDeleteEvent:
				t.handleServerDelete(ctx, e.Server)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (t *control) startServerRunners(ctx context.Context) error {
	servers, err := dao.ChatServers.GetAll(t.store)
	if err != nil {
		return err
	}

	for _, server := range servers {
		t.startServerRunner(ctx, server)
	}
	return nil
}

func (t *control) handleServerChange(ctx context.Context, server *chatv1.Server) {
	t.lock.Lock()
	defer t.lock.Unlock()
	if !t.runners.Has(&runner{key: server.Key.Public}) {
		t.startServerRunner(ctx, server)
	}
}

func (t *control) handleServerDelete(ctx context.Context, server *chatv1.Server) {
	t.lock.Lock()
	defer t.lock.Unlock()
	t.stopServerRunner(ctx, server)
}

func (t *control) startServerRunner(ctx context.Context, server *chatv1.Server) {
	runner, err := newRunner(ctx, t.logger, t.vpn, t.store, t.observers, t.network.Dialer(), t.transfer, t.directory, server.Key.Public, server.NetworkKey, server)
	if err != nil {
		t.logger.Error("failed to start chat runner",
			logutil.ByteHex("chat", server.Key.Public),
			logutil.ByteHex("network", server.NetworkKey),
			zap.Uint64("serverID", server.Id),
			zap.Error(err),
		)
		return
	}
	t.runners.ReplaceOrInsert(runner)
}

func (t *control) stopServerRunner(ctx context.Context, server *chatv1.Server) {
	runner, ok := t.runners.Delete(&runner{key: server.Key.Public}).(*runner)
	if !ok {
		runner.Close()
	}
}

func (t *control) SyncAssets(serverID uint64, forceUnifiedUpdate bool) error {
	t.observers.EmitGlobal(&chatv1.SyncAssetsEvent{
		ServerId:           serverID,
		ForceUnifiedUpdate: forceUnifiedUpdate,
	})
	return nil
}

func (t *control) client(ctx context.Context, networkKey, key []byte) (*network.RPCClient, *chatv1.ChatClient, error) {
	client, err := t.network.Dialer().Client(ctx, networkKey, key, ServiceAddressSalt)
	if err != nil {
		return nil, nil, err
	}

	return client, chatv1.NewChatClient(client), nil
}

// ReadServer ...
func (t *control) ReadServer(ctx context.Context, networkKey, key []byte) (<-chan *chatv1.ServerEvent, <-chan *chatv1.AssetBundle, error) {
	logger := t.logger.With(
		logutil.ByteHex("chat", key),
		logutil.ByteHex("network", networkKey),
	)

	t.lock.Lock()
	defer t.lock.Unlock()

	runner, ok := t.runners.Get(&runner{key: key}).(*runner)
	if !ok {
		var err error
		runner, err = newRunner(ctx, t.logger, t.vpn, t.store, t.observers, t.network.Dialer(), t.transfer, t.directory, key, networkKey, nil)
		if err != nil {
			logger.Error("failed to start chat runner", zap.Error(err))
			return nil, nil, err
		}
		t.runners.ReplaceOrInsert(runner)
	}

	events := make(chan *chatv1.ServerEvent)
	assets := make(chan *chatv1.AssetBundle)

	go func() {
		defer close(events)
		defer close(assets)

		for {
			eg, rctx := errgroup.WithContext(ctx)

			readers, stop, err := runner.Reader(rctx)
			if err != nil {
				logger.Debug("open chat readers failed", zap.Error(err))
				return
			}
			defer stop()

			eg.Go(func() error {
				for {
					e := &chatv1.ServerEvent{}
					err := readers.events.Read(e)
					if err == protoutil.ErrShortRead {
						continue
					} else if err != nil {
						return fmt.Errorf("reading event: %w", err)
					}

					select {
					case events <- e:
					case <-ctx.Done():
						return ctx.Err()
					}
				}
			})

			eg.Go(func() error {
				for {
					b := &chatv1.AssetBundle{}
					err := readers.assets.Read(b)
					if err != nil {
						return fmt.Errorf("reading asset bundle: %w", err)
					}

					select {
					case assets <- b:
					case <-ctx.Done():
						return ctx.Err()
					}
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
func (t *control) SendMessage(ctx context.Context, networkKey, key []byte, m string) error {
	c, cc, err := t.client(ctx, networkKey, key)
	if err != nil {
		return err
	}
	defer c.Close()

	return cc.SendMessage(ctx, &chatv1.SendMessageRequest{Body: m}, &chatv1.SendMessageResponse{})
}
