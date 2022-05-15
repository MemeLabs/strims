// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"context"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/protoutil"
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
	events, assets  *protoutil.ChunkStreamReader
	checkpointCache chan struct{}
}

func (r readers) CheckpointCache() {
	select {
	case r.checkpointCache <- struct{}{}:
	default:
	}
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
	return newChatReader(s.logger, s.store, s.transfer, s.key, s.networkKey)
}

func (s *runnerAdapter) Server() (servicemanager.Readable[readers], error) {
	if s.config == nil {
		return nil, nil
	}
	return newChatServer(s.logger, s.store, s.observers, s.dialer, s.transfer, s.directory, s.config)
}
