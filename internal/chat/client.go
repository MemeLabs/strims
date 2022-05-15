// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"context"
	"fmt"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"go.uber.org/zap"
)

func newChatReader(
	logger *zap.Logger,
	store *dao.ProfileStore,
	transfer transfer.Control,
	key []byte,
	networkKey []byte,
) (*chatReader, error) {
	eventSwarmOptions := ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%x_events", key[:8])}
	eventSwarmOptions.Assign(defaultEventSwarmOptions)
	eventSwarm, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), eventSwarmOptions)
	if err != nil {
		return nil, err
	}

	assetSwarmOptions := ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%x_assets", key[:8])}
	assetSwarmOptions.Assign(defaultAssetSwarmOptions)
	assetSwarm, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), assetSwarmOptions)
	if err != nil {
		return nil, err
	}

	cache, err := dao.GetSwarmCache(store, key, AssetsAddressSalt)
	if err == nil {
		if err := assetSwarm.ImportCache(cache); err != nil {
			logger.Debug("cache import failed", zap.Error(err))
		} else {
			logger.Debug(
				"imported chat asset cache",
				zap.Stringer("swarm", assetSwarm.ID()),
				zap.Int("size", len(cache.Data)),
			)
		}
	}

	return &chatReader{
		logger:          logger,
		store:           store,
		transfer:        transfer,
		key:             key,
		networkKey:      networkKey,
		eventSwarm:      eventSwarm,
		assetSwarm:      assetSwarm,
		checkpointCache: make(chan struct{}, 1),
	}, nil
}

type chatReader struct {
	logger          *zap.Logger
	store           *dao.ProfileStore
	transfer        transfer.Control
	key             []byte
	networkKey      []byte
	eventSwarm      *ppspp.Swarm
	assetSwarm      *ppspp.Swarm
	eventReader     *protoutil.ChunkStreamReader
	assetReader     *protoutil.ChunkStreamReader
	checkpointCache chan struct{}
	stopper         servicemanager.Stopper
}

func (d *chatReader) exportCache() error {
	cache, err := d.assetSwarm.ExportCache()
	if err != nil {
		return err
	}
	return dao.SetSwarmCache(d.store, d.assetSwarm.ID(), AssetsAddressSalt, cache)
}

func (d *chatReader) Run(ctx context.Context) error {
	done, ctx := d.stopper.Start(ctx)
	defer done()

	eventTransferID := d.transfer.Add(d.eventSwarm, EventsAddressSalt)
	assetTransferID := d.transfer.Add(d.assetSwarm, AssetsAddressSalt)
	d.transfer.Publish(eventTransferID, d.networkKey)
	d.transfer.Publish(assetTransferID, d.networkKey)

CheckpointLoop:
	for {
		select {
		case <-d.checkpointCache:
			if err := d.exportCache(); err != nil {
				d.logger.Debug("cache export failed", zap.Error(err))
			}
		case <-ctx.Done():
			break CheckpointLoop
		}
	}

	d.transfer.Remove(eventTransferID)
	d.transfer.Remove(assetTransferID)
	d.eventSwarm.Close()
	d.assetSwarm.Close()

	return ctx.Err()
}

func (d *chatReader) Close(ctx context.Context) error {
	select {
	case <-d.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (d *chatReader) Reader(ctx context.Context) (readers, error) {
	eventsReader := d.eventSwarm.Reader()
	assetsReader := d.assetSwarm.Reader()
	eventsReader.SetReadStopper(ctx.Done())
	assetsReader.SetReadStopper(ctx.Done())
	eventsReader.Unread()
	assetsReader.Unread()
	return readers{
		events:          protoutil.NewChunkStreamReader(eventsReader, eventChunkSize),
		assets:          protoutil.NewChunkStreamReader(assetsReader, assetChunkSize),
		checkpointCache: d.checkpointCache,
	}, nil
}
