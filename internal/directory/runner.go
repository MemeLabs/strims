package directory

import (
	"bytes"
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
	"github.com/petar/GoLLRB/llrb"
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
		key:     dao.NetworkKey(network),
		network: syncutil.NewPointer(network),

		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		dialer:    dialer,
		transfer:  transfer,
	}

	m, err := servicemanager.New[*protoutil.ChunkStreamReader](logger, ctx, a)
	if err != nil {
		return nil, err
	}

	return &runner{
		key:     dao.NetworkKey(network),
		adapter: a,
		Runner:  m,
	}, nil
}

type runner struct {
	key     []byte
	adapter *runnerAdapter
	*servicemanager.Runner[*protoutil.ChunkStreamReader, *runnerAdapter]
}

func (r *runner) Sync(network *networkv1.Network) {
	r.adapter.network.Swap(network)
}

func (r *runner) Less(o llrb.Item) bool {
	if o, ok := o.(*runner); ok {
		return bytes.Compare(r.key, o.key) == -1
	}
	return !o.Less(r)
}

func (r *runner) Logger() *zap.Logger {
	return r.adapter.logger
}

type runnerAdapter struct {
	key     []byte
	network syncutil.Pointer[networkv1.Network]

	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    network.Dialer
	transfer  transfer.Control
}

func (s *runnerAdapter) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, s.network.Get().Id)
}

func (s *runnerAdapter) Client() (servicemanager.Readable[*protoutil.ChunkStreamReader], error) {
	return newDirectoryReader(s.logger, s.transfer, s.key)
}

func (s *runnerAdapter) Server() (servicemanager.Readable[*protoutil.ChunkStreamReader], error) {
	if s.network.Get().GetServerConfig() == nil {
		return nil, nil
	}
	return newDirectoryServer(s.logger, s.vpn, s.store, s.dialer, s.transfer, s.observers, s.network.Get())
}
