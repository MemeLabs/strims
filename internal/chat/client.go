// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"context"
	"fmt"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/servicemanager"
	"github.com/MemeLabs/strims/internal/transfer"
	"github.com/MemeLabs/strims/pkg/options"
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
	eventSwarmOptions := options.AssignDefaults(ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%.8s_events", ppspp.SwarmID(key))}, defaultEventSwarmOptions)
	eventSwarm, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), eventSwarmOptions)
	if err != nil {
		return nil, err
	}

	assetSwarmOptions := options.AssignDefaults(ppspp.SwarmOptions{Label: fmt.Sprintf("chat_%.8s_assets", ppspp.SwarmID(key))}, defaultAssetSwarmOptions)
	assetSwarm, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), assetSwarmOptions)
	if err != nil {
		return nil, err
	}

	cache, err := dao.GetSwarmCache(store, key, AssetsSwarmSalt)
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
	checkpointCache chan struct{}
	stopper         servicemanager.Stopper
}

func (d *chatReader) exportCache() error {
	cache, err := d.assetSwarm.ExportCache()
	if err != nil {
		return err
	}
	return dao.SetSwarmCache(d.store, d.assetSwarm.ID(), AssetsSwarmSalt, cache)
}

func (d *chatReader) Run(ctx context.Context) error {
	done, ctx := d.stopper.Start(ctx)
	defer done()

	eventTransferID := d.transfer.Add(d.eventSwarm, EventsSwarmSalt)
	assetTransferID := d.transfer.Add(d.assetSwarm, AssetsSwarmSalt)
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
	eventReader := d.eventSwarm.Reader()
	assetReader := d.assetSwarm.Reader()
	eventReader.SetReadStopper(ctx.Done())
	assetReader.SetReadStopper(ctx.Done())
	eventReader.Unread()
	assetReader.Unread()
	return readers{
		events:          protoutil.NewChunkStreamReader(eventReader, eventChunkSize),
		assets:          protoutil.NewChunkStreamReader(assetReader, assetChunkSize),
		checkpointCache: d.checkpointCache,
	}, nil
}
