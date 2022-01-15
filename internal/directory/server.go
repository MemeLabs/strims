package directory

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

func newDirectoryServer(
	logger *zap.Logger,
	dialer network.Dialer,
	network *networkv1.Network,
) (*directoryServer, error) {
	config := network.GetServerConfig()
	if config == nil {
		return nil, errors.New("directory server requires network root key")
	}

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: swarmOptions,
		Key:          config.Key,
	})
	if err != nil {
		return nil, err
	}

	ew, err := protoutil.NewChunkStreamWriter(w, chunkSize)
	if err != nil {
		return nil, err
	}

	s := &directoryServer{
		logger:      logger,
		config:      config,
		swarm:       w.Swarm(),
		service:     newDirectoryService(logger, dialer, config.Key, config.Directory, ew),
		eventReader: protoutil.NewChunkStreamReader(w.Swarm().Reader(), chunkSize),
	}
	return s, nil
}

type directoryServer struct {
	logger      *zap.Logger
	config      *networkv1.ServerConfig
	swarm       *ppspp.Swarm
	service     *directoryService
	eventReader *protoutil.ChunkStreamReader
	cancel      context.CancelFunc
}

func (s *directoryServer) Run(ctx context.Context, dialer network.Dialer, transfer transfer.Control) error {
	ctx, cancel := context.WithCancel(ctx)
	s.cancel = cancel

	transferID := transfer.Add(s.swarm, AddressSalt)
	transfer.Publish(transferID, s.config.Key.Public)

	server, err := dialer.Server(ctx, s.config.Key.Public, s.config.Key, AddressSalt)
	if err != nil {
		return err
	}

	networkv1directory.RegisterDirectoryService(server, s.service)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	transfer.Remove(transferID)
	s.swarm.Close()

	s.cancel = nil

	return err
}

func (s *directoryServer) Close() {
	if s == nil || s.cancel == nil {
		return
	}
	s.cancel()
}
