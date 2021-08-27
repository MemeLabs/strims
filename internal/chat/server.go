package chat

import (
	"context"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
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

var eventSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          256,
	LiveWindow:         16 * 1024,
	ChunksPerSignature: 1,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
}

var assetSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:          1024,
	LiveWindow:         16 * 1024, // caps the bundle size at 16mb...
	ChunksPerSignature: 128,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodMerkleTree,
	},
	Scheduler: ppspp.SchedulerOptions{
		HackReadAll: true,
	},
}

var eventChunkSize = eventSwarmOptions.ChunkSize
var assetChunkSize = assetSwarmOptions.ChunkSize * assetSwarmOptions.ChunksPerSignature

func getServerConfig(store kv.Store, id uint64) (*chatv1.Server, []*chatv1.Emote, error) {
	var config *chatv1.Server
	var emotes []*chatv1.Emote
	err := store.View(func(tx kv.Tx) (err error) {
		config, err = dao.GetChatServer(tx, id)
		if err != nil {
			return
		}
		emotes, err = dao.GetChatEmotes(tx, id)
		return
	})
	return config, emotes, err
}

func newChatServer(
	logger *zap.Logger,
	store kv.Store,
	config *chatv1.Server,
) (*chatServer, error) {
	eventSwarm, eventWriter, err := newWriter(config.Key, eventSwarmOptions)
	if err != nil {
		return nil, err
	}

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
		eventReader:    protoutil.NewChunkStreamReader(eventSwarm.Reader(), eventChunkSize),
		assetReader:    protoutil.NewChunkStreamReader(assetSwarm.Reader(), assetChunkSize),
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
	eventReader    *protoutil.ChunkStreamReader
	assetReader    *protoutil.ChunkStreamReader
	cancel         context.CancelFunc
}

func (s *chatServer) Run(
	ctx context.Context,
	dialer control.DialerControl,
	transfer control.TransferControl,
) error {
	ctx, cancel := context.WithCancel(ctx)
	s.cancel = cancel

	eventTransferID := transfer.Add(s.eventSwarm, EventsAddressSalt)
	assetTransferID := transfer.Add(s.assetSwarm, AssetsAddressSalt)
	transfer.Publish(eventTransferID, s.config.NetworkKey)
	transfer.Publish(assetTransferID, s.config.NetworkKey)

	server, err := dialer.Server(s.config.NetworkKey, s.config.Key, ServiceAddressSalt)
	if err != nil {
		return err
	}

	chatv1.RegisterChatService(server, s.service)

	config, emotes, err := getServerConfig(s.store, s.config.Id)
	if err != nil {
		return err
	}
	s.service.Sync(config)
	s.assetPublisher.Sync(config, emotes)

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
	config, emotes, err := getServerConfig(s.store, s.config.Id)
	if err != nil {
		return
	}

	s.service.Sync(config)
	s.assetPublisher.Sync(config, emotes)
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

func (s *assetPublisher) Sync(config *chatv1.Server, emotes []*chatv1.Emote) error {
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

	delete(removed, config.Id)
	c := dao.CRC32Message(config)
	if c != s.checksums[config.Id] {
		s.checksums[config.Id] = c
		b.Room = config.Room
	}

	for id := range removed {
		b.RemovedEmotes = append(b.RemovedEmotes, id)
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
