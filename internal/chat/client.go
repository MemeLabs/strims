package chat

import (
	"context"
	"fmt"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
)

func newChatReader(logger *zap.Logger, key, networkKey []byte) (*chatReader, error) {
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
		key:        key,
		networkKey: networkKey,
		eventSwarm: eventSwarm,
		assetSwarm: assetSwarm,
	}, nil
}

type chatReader struct {
	logger      *zap.Logger
	key         []byte
	networkKey  []byte
	eventSwarm  *ppspp.Swarm
	assetSwarm  *ppspp.Swarm
	eventReader *protoutil.ChunkStreamReader
	assetReader *protoutil.ChunkStreamReader
	cancel      context.CancelFunc
}

func (d *chatReader) Run(ctx context.Context, transfer control.TransferControl) error {
	ctx, cancel := context.WithCancel(ctx)
	d.cancel = cancel

	eventTransferID := transfer.Add(d.eventSwarm, EventsAddressSalt)
	assetTransferID := transfer.Add(d.assetSwarm, AssetsAddressSalt)
	transfer.Publish(eventTransferID, d.networkKey)
	transfer.Publish(assetTransferID, d.networkKey)

	<-ctx.Done()

	transfer.Remove(eventTransferID)
	transfer.Remove(assetTransferID)
	d.eventSwarm.Close()
	d.assetSwarm.Close()

	d.cancel = nil

	return ctx.Err()
}

func (d *chatReader) Close() {
	if d == nil || d.cancel == nil {
		return
	}
	d.cancel()
}

func (d *chatReader) Readers() (events, assets *protoutil.ChunkStreamReader) {
	events = protoutil.NewChunkStreamReader(d.eventSwarm.Reader(), eventChunkSize)
	assets = protoutil.NewChunkStreamReader(d.assetSwarm.Reader(), assetChunkSize)
	return
}
