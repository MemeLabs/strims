// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ca

import (
	"context"

	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"go.uber.org/zap"
)

func newCAReader(logger *zap.Logger, transfer transfer.Control, key []byte) (*reader, error) {
	s, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), caSwarmOptions)
	if err != nil {
		return nil, err
	}

	return &reader{
		logger:   logger,
		transfer: transfer,
		key:      key,
		swarm:    s,
	}, nil
}

type reader struct {
	logger      *zap.Logger
	transfer    transfer.Control
	key         []byte
	swarm       *ppspp.Swarm
	eventReader *protoutil.ChunkStreamReader
	stopper     servicemanager.Stopper
}

func (d *reader) Reader(ctx context.Context) (*protoutil.ChunkStreamReader, error) {
	reader := d.swarm.Reader()
	reader.SetReadStopper(ctx.Done())
	return protoutil.NewChunkStreamReader(reader, caSwarmOptions.ChunkSize), nil
}

func (d *reader) Run(ctx context.Context) error {
	done, ctx := d.stopper.Start(ctx)
	defer done()

	transferID := d.transfer.Add(d.swarm, AddressSalt)
	d.transfer.Publish(transferID, d.key)

	<-ctx.Done()

	d.transfer.Remove(transferID)
	d.swarm.Close()

	return ctx.Err()
}

func (d *reader) Close(ctx context.Context) error {
	select {
	case <-d.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}
