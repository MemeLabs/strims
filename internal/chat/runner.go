package chat

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
)

func newRunner(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer network.Dialer,
	transfer transfer.Control,
	directory directory.Control,
	key []byte,
	networkKey []byte,
	config *chatv1.Server,
) (*runner, error) {
	logger = logger.With(logutil.ByteHex("chat", key))

	a := &runnerAdapter{
		logger:    logger,
		store:     store,
		observers: observers,
		dialer:    dialer,
		transfer:  transfer,
		directory: directory,

		key:        key,
		networkKey: networkKey,
		config:     config,
	}

	m, err := servicemanager.New[readers](logger, ctx, a)
	if err != nil {
		return nil, err
	}

	return &runner{
		key:    key,
		Runner: m,
	}, nil
}

type runner struct {
	key []byte
	*servicemanager.Runner[readers, *runnerAdapter]
}

type readers struct {
	events, assets *protoutil.ChunkStreamReader
}

type runnerAdapter struct {
	logger    *zap.Logger
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    network.Dialer
	transfer  transfer.Control
	directory directory.Control

	key        []byte
	networkKey []byte
	config     *chatv1.Server
}

func (s *runnerAdapter) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, "chat", s.config.Id)
}

func (s *runnerAdapter) Client() (servicemanager.Readable[readers], error) {
	return newChatReader(s.logger, s.transfer, s.key, s.networkKey)
}

func (s *runnerAdapter) Server() (servicemanager.Readable[readers], error) {
	if s.config == nil {
		return nil, nil
	}
	return newChatServer(s.logger, s.store, s.observers, s.dialer, s.transfer, s.directory, s.config)
}
