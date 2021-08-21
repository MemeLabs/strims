package chat

import (
	"context"

	control "github.com/MemeLabs/go-ppspp/internal"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
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
}

var eventChunkSize = eventSwarmOptions.ChunkSize
var assetChunkSize = assetSwarmOptions.ChunkSize * assetSwarmOptions.ChunksPerSignature

func newChatServer(
	logger *zap.Logger,
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
		logger:      logger,
		config:      config,
		eventsSwarm: eventSwarm,
		assetsSwarm: assetSwarm,
		service:     newChatService(logger, config.Key, eventWriter),
		eventReader: protoutil.NewChunkStreamReader(eventSwarm.Reader(), eventChunkSize),
		assetWriter: assetWriter,
		assetReader: protoutil.NewChunkStreamReader(eventSwarm.Reader(), assetChunkSize),
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
	logger      *zap.Logger
	config      *chatv1.Server
	eventsSwarm *ppspp.Swarm
	assetsSwarm *ppspp.Swarm
	service     *chatService
	eventReader *protoutil.ChunkStreamReader
	assetWriter *protoutil.ChunkStreamWriter
	assetReader *protoutil.ChunkStreamReader
	cancel      context.CancelFunc
}

func (s *chatServer) Run(
	ctx context.Context,
	dialer control.DialerControl,
	transfer control.TransferControl,
) error {
	ctx, cancel := context.WithCancel(ctx)
	s.cancel = cancel

	eventsTransferID := transfer.Add(s.eventsSwarm, EventsAddressSalt)
	assetsTransferID := transfer.Add(s.assetsSwarm, AssetsAddressSalt)
	transfer.Publish(eventsTransferID, s.config.NetworkKey)
	transfer.Publish(assetsTransferID, s.config.NetworkKey)

	server, err := dialer.Server(s.config.NetworkKey, s.config.Key, ServiceAddressSalt)
	if err != nil {
		return err
	}

	chatv1.RegisterChatService(server, s.service)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	transfer.Remove(eventsTransferID)
	transfer.Remove(assetsTransferID)
	s.eventsSwarm.Close()
	s.assetsSwarm.Close()

	s.cancel = nil

	return err
}

func (s *chatServer) Close() {
	if s == nil || s.cancel == nil {
		return
	}
	s.cancel()
}

func (s *chatServer) PublishAssets(asset *chatv1.AssetBundle) error {
	return s.assetWriter.Write(asset)
}
