// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"context"
	"errors"
	"fmt"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

var ServiceAddressSalt = []byte("chat")
var EventsAddressSalt = []byte("chat:events")
var AssetsAddressSalt = []byte("chat:assets")

var defaultEventSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          256,
	LiveWindow:         1024, // 256kb
	ChunksPerSignature: 1,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
	DeliveryMode: ppspp.BestEffortDeliveryMode,
}

var defaultAssetSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          1024,
	LiveWindow:         32 * 1024, // caps the bundle size at 32mb...
	ChunksPerSignature: 128,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodMerkleTree,
	},
	DeliveryMode: ppspp.MandatoryDeliveryMode,
	BufferLayout: store.ElasticBufferLayout,
}

var eventChunkSize = defaultEventSwarmOptions.ChunkSize
var assetChunkSize = defaultAssetSwarmOptions.ChunkSize * defaultAssetSwarmOptions.ChunksPerSignature
var assetWindowSize = defaultAssetSwarmOptions.ChunkSize * defaultAssetSwarmOptions.LiveWindow

func getServerConfig(store kv.Store, id uint64) (config *chatv1.Server, icon *chatv1.ServerIcon, emotes []*chatv1.Emote, modifiers []*chatv1.Modifier, tags []*chatv1.Tag, err error) {
	err = store.View(func(tx kv.Tx) (err error) {
		config, err = dao.ChatServers.Get(tx, id)
		if err != nil {
			return
		}
		icon, err = dao.ChatServerIcons.Get(tx, id)
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return
		}
		emotes, err = dao.GetChatEmotesByServerID(tx, id)
		if err != nil {
			return
		}
		modifiers, err = dao.GetChatModifiersByServerID(tx, id)
		if err != nil {
			return
		}
		tags, err = dao.GetChatTagsByServerID(tx, id)
		if err != nil {
			return
		}
		return
	})
	return
}

func newChatServer(
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer network.Dialer,
	transfer transfer.Control,
	directory directory.Control,
	config *chatv1.Server,
) (*chatServer, error) {
	eventSwarmOptions := ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%s_events", ppspp.SwarmID(config.Key.Public[:8]))}
	eventSwarmOptions.Assign(defaultEventSwarmOptions)
	eventSwarm, eventWriter, err := newWriter(config.Key, eventSwarmOptions)
	if err != nil {
		return nil, err
	}

	assetSwarmOptions := ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%s_assets", ppspp.SwarmID(config.Key.Public[:8]))}
	assetSwarmOptions.Assign(defaultAssetSwarmOptions)
	assetSwarm, assetWriter, err := newWriter(config.Key, assetSwarmOptions)
	if err != nil {
		return nil, err
	}

	s := &chatServer{
		logger:         logger,
		store:          store,
		observers:      observers,
		dialer:         dialer,
		transfer:       transfer,
		directory:      directory,
		config:         config,
		eventSwarm:     eventSwarm,
		assetSwarm:     assetSwarm,
		service:        newChatService(logger, eventWriter, observers, store, config),
		assetPublisher: newAssetPublisher(logger, assetWriter),
	}

	return s, nil
}

func newWriter(k *key.Key, opt ppspp.SwarmOptions) (*ppspp.Swarm, *protoutil.ChunkStreamWriter, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: opt,
		Key:          k,
	})
	if err != nil {
		return nil, nil, err
	}

	ew, err := protoutil.NewChunkStreamWriter(w, opt.ChunkSize*opt.ChunksPerSignature)
	if err != nil {
		return nil, nil, err
	}

	return w.Swarm(), ew, nil
}

type chatServer struct {
	logger         *zap.Logger
	store          *dao.ProfileStore
	observers      *event.Observers
	dialer         network.Dialer
	transfer       transfer.Control
	directory      directory.Control
	config         *chatv1.Server
	eventSwarm     *ppspp.Swarm
	assetSwarm     *ppspp.Swarm
	service        *chatService
	assetPublisher *assetPublisher
	stopper        servicemanager.Stopper
}

func (s *chatServer) Run(ctx context.Context) error {
	done, ctx := s.stopper.Start(ctx)
	defer done()

	eventTransferID := s.transfer.Add(s.eventSwarm, EventsAddressSalt)
	assetTransferID := s.transfer.Add(s.assetSwarm, AssetsAddressSalt)
	s.transfer.Publish(eventTransferID, s.config.NetworkKey)
	s.transfer.Publish(assetTransferID, s.config.NetworkKey)

	server, err := s.dialer.Server(ctx, s.config.NetworkKey, s.config.Key, ServiceAddressSalt)
	if err != nil {
		return err
	}

	chatv1.RegisterChatService(server, s.service)

	go s.watchAssets(ctx)
	s.syncAssets(true)

	listingID, err := s.directory.Publish(
		ctx,
		&networkv1directory.Listing{
			Content: &networkv1directory.Listing_Chat_{
				Chat: &networkv1directory.Listing_Chat{
					Key:  s.config.Key.Public,
					Name: s.config.Room.Name,
				},
			},
		},
		s.config.NetworkKey,
	)
	if err != nil {
		return fmt.Errorf("publishing chat server to directory failed: %w", err)
	}

	s.service.SetListingID(listingID)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	s.transfer.Remove(eventTransferID)
	s.transfer.Remove(assetTransferID)
	s.eventSwarm.Close()
	s.assetSwarm.Close()

	if err := s.directory.Unpublish(context.Background(), listingID, s.config.NetworkKey); err != nil {
		s.logger.Info("unpublishing chat server from directory failed", zap.Error(err))
	}

	return err
}

func (s *chatServer) Close(ctx context.Context) error {
	select {
	case <-s.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (s *chatServer) Reader(ctx context.Context) (readers, error) {
	eventsReader := s.eventSwarm.Reader()
	assetsReader := s.assetSwarm.Reader()
	eventsReader.SetReadStopper(ctx.Done())
	assetsReader.SetReadStopper(ctx.Done())
	eventsReader.Unread()
	assetsReader.Unread()
	return readers{
		events: protoutil.NewChunkStreamReader(eventsReader, eventChunkSize),
		assets: protoutil.NewChunkStreamReader(assetsReader, assetChunkSize),
	}, nil
}

func (s *chatServer) watchAssets(ctx context.Context) {
	events, done := s.observers.Events()
	defer done()

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case *chatv1.ServerChangeEvent:
				s.service.SyncConfig(e.Server)
			case *chatv1.ServerIconChangeEvent:
				s.trySyncAssets(e.ServerIcon.Id, false)
			case *chatv1.SyncAssetsEvent:
				s.trySyncAssets(e.ServerId, e.ForceUnifiedUpdate)
			case *chatv1.EmoteChangeEvent:
				s.trySyncAssets(e.Emote.ServerId, false)
			case *chatv1.EmoteDeleteEvent:
				s.trySyncAssets(e.Emote.ServerId, false)
			case *chatv1.ModifierChangeEvent:
				s.trySyncAssets(e.Modifier.ServerId, false)
			case *chatv1.ModifierDeleteEvent:
				s.trySyncAssets(e.Modifier.ServerId, false)
			case *chatv1.TagChangeEvent:
				s.trySyncAssets(e.Tag.ServerId, false)
			case *chatv1.TagDeleteEvent:
				s.trySyncAssets(e.Tag.ServerId, false)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (s *chatServer) trySyncAssets(serverID uint64, unifiedUpdate bool) {
	if serverID == s.config.Id {
		go s.syncAssets(unifiedUpdate)
	}
}

func (s *chatServer) syncAssets(unifiedUpdate bool) {
	logger := s.logger.With(zap.Uint64("serverID", s.config.Id))
	logger.Debug("syncing assets for chat server")

	config, icon, emotes, modifiers, tags, err := getServerConfig(s.store, s.config.Id)
	if err != nil {
		return
	}

	if err := s.service.Sync(config, emotes, modifiers, tags); err != nil {
		logger.Error("syncing assets to service failed", zap.Error(err))
	}
	if err := s.assetPublisher.Sync(unifiedUpdate, config, icon, emotes, modifiers, tags); err != nil {
		logger.Error("syncing assets to asset stream failed", zap.Error(err))
	}
}
