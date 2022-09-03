// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"context"
	"sync/atomic"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/vpn"
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
	a := &runnerAdapter{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		dialer:    dialer,
		transfer:  transfer,

		network: syncutil.NewPointer(network),
	}

	m, err := servicemanager.New[readers](logger, ctx, a)
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

func (r *runner) Sync(network *networkv1.Network) {
	r.adapter.network.Swap(network)
}

func (r *runner) NetworkKey() []byte {
	return dao.NetworkKey(r.adapter.network.Load())
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

	network atomic.Pointer[networkv1.Network]
}

func (s *runnerAdapter) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, "directory", s.network.Load().Id)
}

func (s *runnerAdapter) Client() (servicemanager.Readable[readers], error) {
	return newDirectoryReader(s.logger, s.transfer, dao.NetworkKey(s.network.Load()))
}

func (s *runnerAdapter) Server() (servicemanager.Readable[readers], error) {
	if s.network.Load().GetServerConfig() == nil {
		return nil, nil
	}
	return newDirectoryServer(s.logger, s.vpn, s.store, s.observers, s.dialer, s.transfer, s.network.Load())
}
