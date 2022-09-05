// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"context"
	"fmt"

	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	"github.com/MemeLabs/strims/pkg/options"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"go.uber.org/zap"
)

func newDirectoryReader(logger *zap.Logger, transfer transfer.Control, key []byte) (*directoryReader, error) {
	eventSwarmOptions := options.AssignDefaults(ppspp.SwarmOptions{Label: fmt.Sprintf("directory_%s_events", ppspp.SwarmID(key[:8]))}, defaultEventSwarmOptions)
	eventSwarm, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), eventSwarmOptions)
	if err != nil {
		return nil, err
	}

	assetSwarmOptions := options.AssignDefaults(ppspp.SwarmOptions{Label: fmt.Sprintf("directory_%s_assets", ppspp.SwarmID(key[:8]))}, defaultAssetSwarmOptions)
	assetSwarm, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), assetSwarmOptions)
	if err != nil {
		return nil, err
	}

	return &directoryReader{
		logger:     logger,
		transfer:   transfer,
		key:        key,
		eventSwarm: eventSwarm,
		assetSwarm: assetSwarm,
	}, nil
}

type directoryReader struct {
	logger     *zap.Logger
	transfer   transfer.Control
	key        []byte
	eventSwarm *ppspp.Swarm
	assetSwarm *ppspp.Swarm
	stopper    servicemanager.Stopper
}

func (d *directoryReader) Run(ctx context.Context) error {
	done, ctx := d.stopper.Start(ctx)
	defer done()

	eventTransferID := d.transfer.Add(d.eventSwarm, EventSwarmSalt)
	assetTransferID := d.transfer.Add(d.assetSwarm, AssetSwarmSalt)
	d.transfer.Publish(eventTransferID, d.key)
	d.transfer.Publish(assetTransferID, d.key)

	<-ctx.Done()

	d.transfer.Remove(eventTransferID)
	d.transfer.Remove(assetTransferID)
	d.eventSwarm.Close()
	d.assetSwarm.Close()

	return ctx.Err()
}

func (d *directoryReader) Close(ctx context.Context) error {
	select {
	case <-d.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (d *directoryReader) Reader(ctx context.Context) (readers, error) {
	eventReader := d.eventSwarm.Reader()
	assetReader := d.assetSwarm.Reader()
	eventReader.SetReadStopper(ctx.Done())
	assetReader.SetReadStopper(ctx.Done())
	eventReader.Unread()
	assetReader.Unread()
	return readers{
		events: protoutil.NewChunkStreamReader(eventReader, eventChunkSize),
		assets: protoutil.NewChunkStreamReader(assetReader, assetChunkSize),
	}, nil
}
