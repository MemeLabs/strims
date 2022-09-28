// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"context"
	"errors"
	"fmt"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/options"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

var (
	AddressSalt    = []byte("directory")
	ConfigSalt     = []byte("directory:config")
	EventSwarmSalt = []byte("directory:events")
	AssetSwarmSalt = []byte("directory:assets")
)

var defaultEventSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          1024,
	LiveWindow:         2 * 1024,
	ChunksPerSignature: 1,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
	DeliveryMode: ppspp.BestEffortDeliveryMode,
}

var defaultAssetSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          1024,
	LiveWindow:         2 * 1024,
	ChunksPerSignature: 128,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodMerkleTree,
	},
	DeliveryMode: ppspp.MandatoryDeliveryMode,
	BufferLayout: store.ElasticBufferLayout,
}

var eventChunkSize = defaultEventSwarmOptions.ChunkSize
var assetChunkSize = defaultAssetSwarmOptions.ChunkSize * defaultAssetSwarmOptions.ChunksPerSignature

func newDirectoryServer(
	logger *zap.Logger,
	vpn *vpn.Host,
	store dao.Store,
	observers *event.Observers,
	dialer network.Dialer,
	transfer transfer.Control,
	network *networkv1.Network,
) (*directoryServer, error) {
	config := network.GetServerConfig()
	if config == nil {
		return nil, errors.New("directory server requires network root key")
	}

	eventSwarmOptions := options.AssignDefaults(ppspp.SwarmOptions{Label: fmt.Sprintf("directory_%.8s_events", ppspp.SwarmID(config.Key.Public))}, defaultEventSwarmOptions)
	eventSwarm, eventWriter, err := newWriter(config.Key, eventSwarmOptions)
	if err != nil {
		return nil, err
	}

	assetSwarmOptions := options.AssignDefaults(ppspp.SwarmOptions{Label: fmt.Sprintf("directory_%.8s_assets", ppspp.SwarmID(config.Key.Public))}, defaultAssetSwarmOptions)
	assetSwarm, assetWriter, err := newWriter(config.Key, assetSwarmOptions)
	if err != nil {
		return nil, err
	}

	cache, err := dao.GetSwarmCache(store, config.Key.Public, AssetSwarmSalt)
	if err == nil {
		if err := assetSwarm.ImportCache(cache); err != nil {
			logger.Debug("cache import failed", zap.Error(err))
		} else {
			logger.Debug(
				"imported chat asset cache",
				zap.Stringer("swarm", assetSwarm.ID()),
				zap.Int("size", len(cache.Data)),
			)
		}
	}

	s := &directoryServer{
		logger:      logger,
		store:       store,
		dialer:      dialer,
		transfer:    transfer,
		observers:   observers,
		network:     network,
		eventSwarm:  eventSwarm,
		assetSwarm:  assetSwarm,
		assetWriter: assetWriter,
		service:     newDirectoryService(logger, vpn, store, observers, dialer, network, eventWriter),
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

type directoryServer struct {
	logger      *zap.Logger
	store       dao.Store
	dialer      network.Dialer
	transfer    transfer.Control
	observers   *event.Observers
	network     *networkv1.Network
	eventSwarm  *ppspp.Swarm
	assetSwarm  *ppspp.Swarm
	assetWriter *protoutil.ChunkStreamWriter
	service     *directoryService
	stopper     servicemanager.Stopper
}

func (s *directoryServer) Reader(ctx context.Context) (readers, error) {
	eventReader := s.eventSwarm.Reader()
	assetReader := s.assetSwarm.Reader()
	eventReader.SetReadStopper(ctx.Done())
	assetReader.SetReadStopper(ctx.Done())
	eventReader.Unread()
	assetReader.Unread()
	return readers{
		events: protoutil.NewChunkStreamReader(eventReader, eventChunkSize),
		assets: protoutil.NewChunkStreamReader(assetReader, assetChunkSize),
	}, nil
}

func (s *directoryServer) Run(ctx context.Context) error {
	done, ctx := s.stopper.Start(ctx)
	defer done()

	eventTransferID := s.transfer.Add(s.eventSwarm, EventSwarmSalt)
	assetTransferID := s.transfer.Add(s.assetSwarm, AssetSwarmSalt)
	s.transfer.Publish(eventTransferID, s.network.ServerConfig.Key.Public)
	s.transfer.Publish(assetTransferID, s.network.ServerConfig.Key.Public)

	server, err := s.dialer.Server(ctx, s.network.ServerConfig.Key.Public, s.network.ServerConfig.Key, AddressSalt)
	if err != nil {
		return err
	}

	networkv1directory.RegisterDirectoryService(server, s.service)

	go s.watchAssets(ctx)
	s.syncAssets()

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	s.transfer.Remove(eventTransferID)
	s.transfer.Remove(assetTransferID)
	s.eventSwarm.Close()
	s.assetSwarm.Close()

	return err
}

func (s *directoryServer) Close(ctx context.Context) error {
	select {
	case <-s.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (s *directoryServer) watchAssets(ctx context.Context) {
	events, done := s.observers.Events()
	defer done()

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case *networkv1.NetworkChangeEvent:
				s.trySyncAssets(e.Network.Id)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (s *directoryServer) trySyncAssets(serverID uint64) {
	if serverID == s.network.Id {
		go s.syncAssets()
	}
}

func (s *directoryServer) syncAssets() {
	s.logger.Debug("syncing assets for directory server")

	network, err := dao.Networks.Get(s.store, s.network.Id)
	if err != nil {
		return
	}

	config := network.GetServerConfig().GetDirectory()
	bundle := &networkv1directory.AssetBundle{
		Icon: network.GetServerConfig().GetIcon(),
		Directory: &networkv1directory.ClientConfig{
			Integrations: &networkv1directory.ClientConfig_Integrations{
				Angelthump: config.GetIntegrations().GetAngelthump().GetEnable(),
				Twitch:     config.GetIntegrations().GetTwitch().GetEnable(),
				Youtube:    config.GetIntegrations().GetYoutube().GetEnable(),
				Swarm:      config.GetIntegrations().GetSwarm().GetEnable(),
			},
			PublishQuota:    config.GetPublishQuota(),
			JoinQuota:       config.GetJoinQuota(),
			MinPingInterval: config.GetMinPingInterval(),
			MaxPingInterval: config.GetMaxPingInterval(),
		},
	}

	s.assetWriter.Reset()

	if err := s.assetWriter.Write(bundle); err != nil {
		s.logger.Error("syncing assets to asset stream failed", zap.Error(err))
	}
}
