package chat

import (
	"context"
	"fmt"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

var ServiceAddressSalt = []byte("chat")
var EventsAddressSalt = []byte("chat:events")
var AssetsAddressSalt = []byte("chat:assets")

var defaultEventSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          256,
	LiveWindow:         16 * 1024,
	ChunksPerSignature: 1,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
	DeliveryMode: ppspp.BestEffortDeliveryMode,
}

var defaultAssetSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          1024,
	LiveWindow:         16 * 1024, // caps the bundle size at 16mb...
	ChunksPerSignature: 128,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodMerkleTree,
	},
	DeliveryMode: ppspp.MandatoryDeliveryMode,
}

var eventChunkSize = defaultEventSwarmOptions.ChunkSize
var assetChunkSize = defaultAssetSwarmOptions.ChunkSize * defaultAssetSwarmOptions.ChunksPerSignature

func getServerConfig(store kv.Store, id uint64) (config *chatv1.Server, emotes []*chatv1.Emote, modifiers []*chatv1.Modifier, tags []*chatv1.Tag, err error) {
	err = store.View(func(tx kv.Tx) (err error) {
		config, err = dao.ChatServers.Get(tx, id)
		if err != nil {
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
	store kv.Store,
	config *chatv1.Server,
) (*chatServer, error) {
	eventSwarmOptions := ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%x_events", config.Key.Public[:8])}
	eventSwarmOptions.Assign(defaultEventSwarmOptions)
	eventSwarm, eventWriter, err := newWriter(config.Key, eventSwarmOptions)
	if err != nil {
		return nil, err
	}

	assetSwarmOptions := ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%x_assets", config.Key.Public[:8])}
	assetSwarmOptions.Assign(defaultAssetSwarmOptions)
	assetSwarm, assetWriter, err := newWriter(config.Key, assetSwarmOptions)
	if err != nil {
		return nil, err
	}

	s := &chatServer{
		logger:         logger,
		store:          store,
		config:         config,
		eventSwarm:     eventSwarm,
		assetSwarm:     assetSwarm,
		service:        newChatService(logger, eventWriter),
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
	store          kv.Store
	config         *chatv1.Server
	eventSwarm     *ppspp.Swarm
	assetSwarm     *ppspp.Swarm
	service        *chatService
	assetPublisher *assetPublisher
	cancel         context.CancelFunc
}

func (s *chatServer) Run(
	ctx context.Context,
	dialer network.Dialer,
	transfer transfer.Control,
) error {
	ctx, cancel := context.WithCancel(ctx)
	s.cancel = cancel

	eventTransferID := transfer.Add(s.eventSwarm, EventsAddressSalt)
	assetTransferID := transfer.Add(s.assetSwarm, AssetsAddressSalt)
	transfer.Publish(eventTransferID, s.config.NetworkKey)
	transfer.Publish(assetTransferID, s.config.NetworkKey)

	server, err := dialer.Server(ctx, s.config.NetworkKey, s.config.Key, ServiceAddressSalt)
	if err != nil {
		return err
	}

	chatv1.RegisterChatService(server, s.service)

	config, emotes, modifiers, tags, err := getServerConfig(s.store, s.config.Id)
	if err != nil {
		return err
	}
	s.service.Sync(config, emotes, modifiers, tags)
	s.assetPublisher.Sync(config, emotes, modifiers, tags)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	transfer.Remove(eventTransferID)
	transfer.Remove(assetTransferID)
	s.eventSwarm.Close()
	s.assetSwarm.Close()

	s.cancel = nil

	return err
}

func (s *chatServer) Close() {
	if s == nil || s.cancel == nil {
		return
	}
	s.cancel()
}

func (s *chatServer) Sync() {
	config, emotes, modifiers, tags, err := getServerConfig(s.store, s.config.Id)
	if err != nil {
		return
	}

	s.service.Sync(config, emotes, modifiers, tags)
	s.assetPublisher.Sync(config, emotes, modifiers, tags)
}

func (s *chatServer) Readers(ctx context.Context) (events, assets *protoutil.ChunkStreamReader) {
	s.eventSwarm.Reader().Unread()
	s.assetSwarm.Reader().Unread()
	s.eventSwarm.Reader().SetReadStopper(ctx.Done())
	s.assetSwarm.Reader().SetReadStopper(ctx.Done())
	events = protoutil.NewChunkStreamReader(s.eventSwarm.Reader(), eventChunkSize)
	assets = protoutil.NewChunkStreamReader(s.assetSwarm.Reader(), assetChunkSize)
	return
}

func newAssetPublisher(logger *zap.Logger, ew *protoutil.ChunkStreamWriter) *assetPublisher {
	return &assetPublisher{
		logger:      logger,
		eventWriter: ew,
		checksums:   map[uint64]uint32{},
	}
}

type assetPublisher struct {
	logger      *zap.Logger
	eventWriter *protoutil.ChunkStreamWriter
	checksums   map[uint64]uint32
	size        int
}

func (s *assetPublisher) Sync(config *chatv1.Server, emotes []*chatv1.Emote, modifiers []*chatv1.Modifier, tags []*chatv1.Tag) error {
	b := &chatv1.AssetBundle{
		IsDelta: len(s.checksums) != 0,
	}

	removed := map[uint64]struct{}{}
	for id := range s.checksums {
		removed[id] = struct{}{}
	}

	for _, e := range emotes {
		delete(removed, e.Id)
		c := dao.CRC32Message(e)
		if c != s.checksums[e.Id] {
			s.checksums[e.Id] = c
			b.Emotes = append(b.Emotes, e)
		}
	}

	for _, e := range modifiers {
		delete(removed, e.Id)
		c := dao.CRC32Message(e)
		if c != s.checksums[e.Id] {
			s.checksums[e.Id] = c
			b.Modifiers = append(b.Modifiers, e)
		}
	}

	for _, e := range tags {
		delete(removed, e.Id)
		c := dao.CRC32Message(e)
		if c != s.checksums[e.Id] {
			s.checksums[e.Id] = c
			b.Tags = append(b.Tags, e)
		}
	}

	delete(removed, config.Id)
	c := dao.CRC32Message(config)
	if c != s.checksums[config.Id] {
		s.checksums[config.Id] = c
		b.Room = config.Room
	}

	for id := range removed {
		b.RemovedIds = append(b.RemovedIds, id)
	}

	// TODO
	// n := s.eventWriter.Size(b)
	// if s.size + n > buffer size {
	// 	reset writer (clear swarm buffer)
	// 	build unified bundle
	// }
	// n.size += n

	s.logger.Debug("writing asset bundle", zap.Int("size", s.eventWriter.Size(b)))

	return s.eventWriter.Write(b)
}
