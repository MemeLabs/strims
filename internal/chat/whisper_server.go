// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/servicemanager"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/logutil"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

var WhisperAddressSalt = []byte("chat:whisper")

func newWhisperServer(
	logger *zap.Logger,
	store dao.Store,
	observers *event.Observers,
	profile *profilev1.Profile,
	dialer network.Dialer,
) (*whisperServer, error) {
	s := &whisperServer{
		logger:    logger,
		store:     store,
		observers: observers,
		profile:   profile,
		dialer:    dialer,

		service:         newWhisperService(logger, store),
		deliveryService: newWhisperDeliveryService(logger, store, dialer),

		serverClosers: map[uint64]context.CancelFunc{},
	}
	return s, nil
}

type whisperServer struct {
	logger    *zap.Logger
	store     dao.Store
	observers *event.Observers
	profile   *profilev1.Profile
	dialer    network.Dialer

	service         *whisperService
	deliveryService *whisperDeliveryService
	stopper         servicemanager.Stopper

	lock          sync.Mutex
	serverClosers map[uint64]context.CancelFunc
}

func (s *whisperServer) Run(ctx context.Context) error {
	done, ctx := s.stopper.Start(ctx)
	defer done()

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.deliveryService.Run(ctx) })
	eg.Go(func() error { return s.startServers(ctx, eg) })

	events, done := s.observers.Events()
	defer done()

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case *chatv1.WhisperRecordChangeEvent:
				s.deliveryService.HandleWhisper(e.WhisperRecord)
			case event.NetworkStart:
				eg.Go(func() error { return s.startServer(ctx, e.Network) })
			case event.NetworkStop:
				go s.stopServer(e.Network)
			}
		case <-ctx.Done():
			return eg.Wait()
		}
	}
}

func (s *whisperServer) startServers(ctx context.Context, eg *errgroup.Group) error {
	networks, err := dao.Networks.GetAll(s.store)
	if err != nil {
		return err
	}

	for _, n := range networks {
		n := n
		eg.Go(func() error { return s.startServer(ctx, n) })
	}
	return nil
}

func (s *whisperServer) startServer(ctx context.Context, network *networkv1.Network) error {
	s.lock.Lock()
	if _, ok := s.serverClosers[network.Id]; ok {
		s.lock.Unlock()
		return nil
	}

	ctx, cancel := context.WithCancel(ctx)
	s.serverClosers[network.Id] = cancel
	s.lock.Unlock()

	defer func() {
		s.lock.Lock()
		delete(s.serverClosers, network.Id)
		s.lock.Unlock()
	}()

	server, err := s.dialer.Server(ctx, dao.NetworkKey(network), s.profile.Key, WhisperAddressSalt)
	if err != nil {
		return err
	}

	chatv1.RegisterWhisperService(server, s.service)

	s.logger.Debug(
		"starting whisper server",
		logutil.ByteHex("network", dao.NetworkKey(network)),
	)

	err = server.Listen(ctx)
	if err != nil && !errors.Is(err, context.Canceled) {
		return err
	}
	return nil
}

func (s *whisperServer) stopServer(network *networkv1.Network) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if close, ok := s.serverClosers[network.Id]; ok {
		close()
	}
}

func (s *whisperServer) Close(ctx context.Context) error {
	select {
	case <-s.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (s *whisperServer) Reader(ctx context.Context) (any, error) {
	return nil, nil
}
