package chat

import (
	"context"
	"fmt"

	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
)

func newChatReader(logger *zap.Logger, transfer transfer.Control, key, networkKey []byte) (*chatReader, error) {
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

	return &chatReader{
		logger:     logger,
		transfer:   transfer,
		key:        key,
		networkKey: networkKey,
		eventSwarm: eventSwarm,
		assetSwarm: assetSwarm,
	}, nil
}

type chatReader struct {
	logger      *zap.Logger
	transfer    transfer.Control
	key         []byte
	networkKey  []byte
	eventSwarm  *ppspp.Swarm
	assetSwarm  *ppspp.Swarm
	eventReader *protoutil.ChunkStreamReader
	assetReader *protoutil.ChunkStreamReader
	stopper     servicemanager.Stopper
}

func (d *chatReader) Run(ctx context.Context) error {
	done, ctx := d.stopper.Start(ctx)
	defer done()

	eventTransferID := d.transfer.Add(d.eventSwarm, EventsAddressSalt)
	assetTransferID := d.transfer.Add(d.assetSwarm, AssetsAddressSalt)
	d.transfer.Publish(eventTransferID, d.networkKey)
	d.transfer.Publish(assetTransferID, d.networkKey)

	<-ctx.Done()

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
		events: protoutil.NewChunkStreamReader(eventsReader, eventChunkSize),
		assets: protoutil.NewChunkStreamReader(assetsReader, assetChunkSize),
	}, nil
}
