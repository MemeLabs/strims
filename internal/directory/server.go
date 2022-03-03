package directory

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

func newDirectoryServer(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer network.Dialer,
	transfer transfer.Control,
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
		dialer:   dialer,
		transfer: transfer,
		key:      config.Key,
		swarm:    w.Swarm(),
		service:  newDirectoryService(logger, vpn, store, observers, dialer, network, ew),
	}
	return s, nil
}

type directoryServer struct {
	dialer      network.Dialer
	transfer    transfer.Control
	key         *key.Key
	swarm       *ppspp.Swarm
	service     *directoryService
	eventReader *protoutil.ChunkStreamReader
	stopper     servicemanager.Stopper
}

func (d *directoryServer) Reader(ctx context.Context) (*protoutil.ChunkStreamReader, error) {
	reader := d.swarm.Reader()
	reader.SetReadStopper(ctx.Done())
	return protoutil.NewChunkStreamReader(reader, chunkSize), nil
}

func (s *directoryServer) Run(ctx context.Context) error {
	done, ctx := s.stopper.Start(ctx)
	defer done()

	transferID := s.transfer.Add(s.swarm, AddressSalt)
	s.transfer.Publish(transferID, s.key.Public)

	server, err := s.dialer.Server(ctx, s.key.Public, s.key, AddressSalt)
	if err != nil {
		return err
	}

	networkv1directory.RegisterDirectoryService(server, s.service)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return s.service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	s.transfer.Remove(transferID)
	s.swarm.Close()

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
