package directory

import (
	"context"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

func newDirectoryServer(
	logger *zap.Logger,
	network *networkv1.Network,
) (*directoryServer, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: swarmOptions,
		Key:          network.Key,
	})
	if err != nil {
		return nil, err
	}

	ew, err := newEventWriter(w)
	if err != nil {
		return nil, err
	}

	s := &directoryServer{
		logger:      logger,
		network:     network,
		swarm:       w.Swarm(),
		service:     newDirectoryService(logger, network.Key, ew),
		eventReader: newEventReader(w.Swarm().Reader()),
	}
	return s, nil
}

type directoryServer struct {
	logger      *zap.Logger
	network     *networkv1.Network
	swarm       *ppspp.Swarm
	service     *directoryService
	eventReader *EventReader
	cancel      context.CancelFunc
}

func (s *directoryServer) Run(ctx context.Context, dialer *dialer.Control, transfer *transfer.Control) error {
	ctx, cancel := context.WithCancel(ctx)
	s.cancel = cancel

	transferID := transfer.Add(s.swarm, AddressSalt)
	transfer.Publish(transferID, s.network.Key.Public)

	server, err := dialer.Server(s.network.Key.Public, s.network.Key, AddressSalt)
	if err != nil {
		return err
	}

	networkv1.RegisterDirectoryService(server, s.service)

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
