package chat

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/hashmap"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

type Control interface {
	Run()
	SyncAssets(serverID uint64, forceUnifiedUpdate bool) error
	ReadServer(ctx context.Context, networkKey, serverKey []byte) (<-chan *chatv1.ServerEvent, <-chan *chatv1.AssetBundle, error)
	SendMessage(ctx context.Context, networkKey, serverKey []byte, m string) error
	Mute(ctx context.Context, networkKey, serverKey, peerKey []byte, duration time.Duration, message string) error
	Unmute(ctx context.Context, networkKey, serverKey, peerKey []byte) error
	GetMute(ctx context.Context, networkKey, serverKey []byte) (*chatv1.GetMuteResponse, error)
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	network network.Control,
	transfer transfer.Control,
	directory directory.Control,
) Control {
	return &control{
		ctx:       ctx,
		logger:    logger,
		store:     store,
		observers: observers,
		network:   network,
		transfer:  transfer,
		directory: directory,

		events:  observers.Chan(),
		runners: hashmap.New[[]byte, *runner](hashmap.NewByteInterface()),
	}
}

// Control ...
type control struct {
	ctx       context.Context
	logger    *zap.Logger
	store     *dao.ProfileStore
	observers *event.Observers
	network   network.Control
	transfer  transfer.Control
	directory directory.Control

	events  chan interface{}
	lock    sync.Mutex
	runners hashmap.Map[[]byte, *runner]
}

// Run ...
func (t *control) Run() {
	if err := t.startServerRunners(); err != nil {
		t.logger.Debug("starting chat server runners failed", zap.Error(err))
	}

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case *chatv1.ServerChangeEvent:
				t.handleServerChange(e.Server)
			case *chatv1.ServerDeleteEvent:
				t.handleServerDelete(e.Server)
			}
		case <-t.ctx.Done():
			return
		}
	}
}

func (t *control) startServerRunners() error {
	servers, err := dao.ChatServers.GetAll(t.store)
	if err != nil {
		return err
	}

	for _, server := range servers {
		t.startServerRunner(server)
	}
	return nil
}

func (t *control) handleServerChange(server *chatv1.Server) {
	t.lock.Lock()
	defer t.lock.Unlock()
	if !t.runners.Has(server.Key.Public) {
		t.startServerRunner(server)
	}
}

func (t *control) handleServerDelete(server *chatv1.Server) {
	t.lock.Lock()
	defer t.lock.Unlock()
	t.stopServerRunner(server)
}

func (t *control) startServerRunner(server *chatv1.Server) {
	runner, err := newRunner(t.ctx, t.logger, t.store, t.observers, t.network.Dialer(), t.transfer, t.directory, server.Key.Public, server.NetworkKey, server)
	if err != nil {
		t.logger.Error("failed to start chat runner",
			logutil.ByteHex("chat", server.Key.Public),
			logutil.ByteHex("network", server.NetworkKey),
			zap.Uint64("serverID", server.Id),
			zap.Error(err),
		)
		return
	}
	t.runners.Set(server.Key.Public, runner)
}

func (t *control) stopServerRunner(server *chatv1.Server) {
	runner, ok := t.runners.Delete(server.Key.Public)
	if ok {
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

// ReadServer ...
func (t *control) ReadServer(ctx context.Context, networkKey, key []byte) (<-chan *chatv1.ServerEvent, <-chan *chatv1.AssetBundle, error) {
	logger := t.logger.With(
		logutil.ByteHex("chat", key),
		logutil.ByteHex("network", networkKey),
	)

	t.lock.Lock()
	defer t.lock.Unlock()

	runner, ok := t.runners.Get(key)
	if !ok {
		var err error
		runner, err = newRunner(ctx, t.logger, t.store, t.observers, t.network.Dialer(), t.transfer, t.directory, key, networkKey, nil)
		if err != nil {
			logger.Error("failed to start chat runner", zap.Error(err))
			return nil, nil, err
		}
		t.runners.Set(key, runner)
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
					if err := readers.events.Read(e); err != nil {
						return fmt.Errorf("reading event: %w", err)
					}

					select {
					case events <- e:
					case <-rctx.Done():
						return rctx.Err()
					}
				}
			})

			eg.Go(func() error {
				for {
					b := &chatv1.AssetBundle{}
					if err := readers.assets.Read(b); err != nil {
						return fmt.Errorf("reading asset bundle: %w", err)
					}

					select {
					case assets <- b:
					case <-rctx.Done():
						return rctx.Err()
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
func (t *control) SendMessage(ctx context.Context, networkKey, serverKey []byte, m string) error {
	client, err := t.network.Dialer().Client(ctx, networkKey, serverKey, ServiceAddressSalt)
	if err != nil {
		return err
	}
	defer client.Close()

	return chatv1.NewChatClient(client).SendMessage(ctx, &chatv1.SendMessageRequest{Body: m}, &chatv1.SendMessageResponse{})
}

func (t *control) Mute(ctx context.Context, networkKey, serverKey, peerKey []byte, duration time.Duration, message string) error {
	client, err := t.network.Dialer().Client(ctx, networkKey, serverKey, ServiceAddressSalt)
	if err != nil {
		return err
	}
	defer client.Close()

	req := &chatv1.MuteRequest{
		PeerKey:      peerKey,
		DurationSecs: uint32(duration / time.Second),
		Message:      message,
	}
	return chatv1.NewChatClient(client).Mute(ctx, req, &chatv1.MuteResponse{})
}

func (t *control) Unmute(ctx context.Context, networkKey, serverKey, peerKey []byte) error {
	client, err := t.network.Dialer().Client(ctx, networkKey, serverKey, ServiceAddressSalt)
	if err != nil {
		return err
	}
	defer client.Close()

	return chatv1.NewChatClient(client).Unmute(ctx, &chatv1.UnmuteRequest{PeerKey: peerKey}, &chatv1.UnmuteResponse{})
}

func (t *control) GetMute(ctx context.Context, networkKey, serverKey []byte) (*chatv1.GetMuteResponse, error) {
	client, err := t.network.Dialer().Client(ctx, networkKey, serverKey, ServiceAddressSalt)
	if err != nil {
		return nil, err
	}
	defer client.Close()

	res := &chatv1.GetMuteResponse{}
	if err := chatv1.NewChatClient(client).GetMute(ctx, &chatv1.GetMuteRequest{}, res); err != nil {
		return nil, err
	}
	return res, nil
}
