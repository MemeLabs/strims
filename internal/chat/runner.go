package chat

import (
	"bytes"
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

func newRunner(ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	dialer network.Dialer,
	transfer transfer.Control,
	directory directory.Control,
	key []byte,
	networkKey []byte,
	config *chatv1.Server,
) *runner {
	logger = logger.With(logutil.ByteHex("chat", key))

	s := &runnerService{
		logger:     logger,
		transfer:   transfer,
		store:      store,
		key:        key,
		networkKey: networkKey,
		config:     config,
	}

	if config != nil {
		server, err := newChatServer(logger, store, dialer, transfer, directory, config)
		if err != nil {
			panic(err)
		}
		s.server = server
	}

	m, err := servicemanager.New[readers](logger, context.Background(), s)
	if err != nil {
		panic(err)
	}

	return &runner{
		key:     key,
		service: s,
		Runner:  m,
	}
}

type runner struct {
	key     []byte
	service *runnerService
	*servicemanager.Runner[readers, *runnerService]
}

func (r *runner) Less(o llrb.Item) bool {
	if o, ok := o.(*runner); ok {
		return bytes.Compare(r.key, o.key) == -1
	}
	return !o.Less(r)
}

func (r *runner) SyncServer() {
	if r.service.server != nil {
		r.service.server.Sync()
	}
}

type readers struct {
	events, assets *protoutil.ChunkStreamReader
}

type runnerService struct {
	logger     *zap.Logger
	transfer   transfer.Control
	store      *dao.ProfileStore
	key        []byte
	networkKey []byte
	config     *chatv1.Server
	server     *chatServer
}

func (s *runnerService) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, s.config.Id)
}

func (s *runnerService) Client() (servicemanager.Readable[readers], error) {
	return newChatReader(s.logger, s.transfer, s.key, s.networkKey)
}

func (s *runnerService) Server() (servicemanager.Readable[readers], error) {
	return s.server, nil
}
