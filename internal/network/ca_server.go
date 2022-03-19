package network

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

func newCAServer(
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer Dialer,
	transfer transfer.Control,
	network *networkv1.Network,
) (*caServer, error) {
	config := network.GetServerConfig()
	if config == nil {
		return nil, errors.New("ca server requires network root key")
	}

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: caSwarmOptions,
		Key:          config.Key,
	})
	if err != nil {
		return nil, err
	}

	ew, err := protoutil.NewChunkStreamWriter(w, caSwarmOptions.ChunkSize)
	if err != nil {
		return nil, err
	}

	s := &caServer{
		dialer:   dialer,
		transfer: transfer,
		key:      config.Key,
		swarm:    w.Swarm(),
		service:  newCAService(logger, store, observers, network, ew),
	}
	return s, nil
}

type caServer struct {
	dialer      Dialer
	transfer    transfer.Control
	key         *key.Key
	swarm       *ppspp.Swarm
	service     *caService
	eventReader *protoutil.ChunkStreamReader
	stopper     servicemanager.Stopper
}

func (d *caServer) Reader(ctx context.Context) (*protoutil.ChunkStreamReader, error) {
	reader := d.swarm.Reader()
	reader.SetReadStopper(ctx.Done())
	return protoutil.NewChunkStreamReader(reader, caSwarmOptions.ChunkSize), nil
}

func (s *caServer) Run(ctx context.Context) error {
	done, ctx := s.stopper.Start(ctx)
	defer done()

	transferID := s.transfer.Add(s.swarm, AddressSalt)
	s.transfer.Publish(transferID, s.key.Public)

	server, err := s.dialer.Server(ctx, s.key.Public, s.key, AddressSalt)
	if err != nil {
		return err
	}

	networkv1ca.RegisterCAService(server, s.service)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	s.transfer.Remove(transferID)
	s.swarm.Close()

	return err
}

func (s *caServer) Close(ctx context.Context) error {
	select {
	case <-s.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}
