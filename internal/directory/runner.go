package directory

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/syncutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

func newRunner(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer network.Dialer,
	transfer transfer.Control,
	network *networkv1.Network,
) (*runner, error) {
	logger = logger.With(logutil.ByteHex("directory", dao.NetworkKey(network)))

	a := &runnerAdapter{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		dialer:    dialer,
		transfer:  transfer,

		network: syncutil.NewPointer(network),
	}

	m, err := servicemanager.New[*protoutil.ChunkStreamReader](logger, ctx, a)
	if err != nil {
		return nil, err
	}

	return &runner{
		adapter: a,
		Runner:  m,
	}, nil
}

type runner struct {
	adapter *runnerAdapter
	*servicemanager.Runner[*protoutil.ChunkStreamReader, *runnerAdapter]
}

func (r *runner) Sync(network *networkv1.Network) {
	r.adapter.network.Swap(network)
}

func (r *runner) NetworkKey() []byte {
	return dao.NetworkKey(r.adapter.network.Get())
}

func (r *runner) Logger() *zap.Logger {
	return r.adapter.logger
}

type runnerAdapter struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    network.Dialer
	transfer  transfer.Control

	network syncutil.Pointer[networkv1.Network]
}

func (s *runnerAdapter) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, "directory", s.network.Get().Id)
}

func (s *runnerAdapter) Client() (servicemanager.Readable[*protoutil.ChunkStreamReader], error) {
	return newDirectoryReader(s.logger, s.transfer, dao.NetworkKey(s.network.Get()))
}

func (s *runnerAdapter) Server() (servicemanager.Readable[*protoutil.ChunkStreamReader], error) {
	if s.network.Get().GetServerConfig() == nil {
		return nil, nil
	}
	return newDirectoryServer(s.logger, s.vpn, s.store, s.observers, s.dialer, s.transfer, s.network.Get())
}
