// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ca

import (
	"context"
	"errors"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network/dialer"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

func newServer(
	logger *zap.Logger,
	store dao.Store,
	observers *event.Observers,
	dialer *dialer.Dialer,
	transfer transfer.Control,
	network *networkv1.Network,
) (*server, error) {
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

	s := &server{
		dialer:   dialer,
		transfer: transfer,
		key:      config.Key,
		swarm:    w.Swarm(),
		service:  newCAService(logger, store, observers, network, ew),
	}
	return s, nil
}

type server struct {
	dialer      *dialer.Dialer
	transfer    transfer.Control
	key         *key.Key
	swarm       *ppspp.Swarm
	service     *service
	eventReader *protoutil.ChunkStreamReader
	stopper     servicemanager.Stopper
}

func (d *server) Reader(ctx context.Context) (*protoutil.ChunkStreamReader, error) {
	reader := d.swarm.Reader()
	reader.SetReadStopper(ctx.Done())
	return protoutil.NewChunkStreamReader(reader, caSwarmOptions.ChunkSize), nil
}

func (s *server) Run(ctx context.Context) error {
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

func (s *server) Close(ctx context.Context) error {
	select {
	case <-s.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}
