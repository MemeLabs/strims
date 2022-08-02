// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ca

import (
	"context"
	"sync/atomic"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network/dialer"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"go.uber.org/zap"
)

func newRunner(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer *dialer.Dialer,
	transfer transfer.Control,
	network *networkv1.Network,
) (*runner, error) {
	logger = logger.With(logutil.ByteHex("network", dao.NetworkKey(network)))

	a := &runnerAdapter{
		logger:    logger,
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

func (r *runner) Logger() *zap.Logger {
	return r.adapter.logger
}

type runnerAdapter struct {
	logger    *zap.Logger
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    *dialer.Dialer
	transfer  transfer.Control

	network atomic.Pointer[networkv1.Network]
}

func (s *runnerAdapter) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, "ca", s.network.Load().Id)
}

func (s *runnerAdapter) Client() (servicemanager.Readable[*protoutil.ChunkStreamReader], error) {
	return newCAReader(s.logger, s.transfer, dao.NetworkKey(s.network.Load()))
}

func (s *runnerAdapter) Server() (servicemanager.Readable[*protoutil.ChunkStreamReader], error) {
	if s.network.Load().GetServerConfig() == nil {
		return nil, nil
	}
	return newServer(s.logger, s.store, s.observers, s.dialer, s.transfer, s.network.Load())
}
