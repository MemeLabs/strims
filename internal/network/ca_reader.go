package network

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
)

func newCAReader(logger *zap.Logger, transfer transfer.Control, key []byte) (*caReader, error) {
	s, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), caSwarmOptions)
	if err != nil {
		return nil, err
	}

	return &caReader{
		logger:   logger,
		transfer: transfer,
		key:      key,
		swarm:    s,
	}, nil
}

type caReader struct {
	logger      *zap.Logger
	transfer    transfer.Control
	key         []byte
	swarm       *ppspp.Swarm
	eventReader *protoutil.ChunkStreamReader
	stopper     servicemanager.Stopper
}

func (d *caReader) Reader(ctx context.Context) (*protoutil.ChunkStreamReader, error) {
	reader := d.swarm.Reader()
	reader.SetReadStopper(ctx.Done())
	return protoutil.NewChunkStreamReader(reader, caSwarmOptions.ChunkSize), nil
}

func (d *caReader) Run(ctx context.Context) error {
	done, ctx := d.stopper.Start(ctx)
	defer done()

	transferID := d.transfer.Add(d.swarm, AddressSalt)
	d.transfer.Publish(transferID, d.key)

	<-ctx.Done()

	d.transfer.Remove(transferID)
	d.swarm.Close()

	return ctx.Err()
}

func (d *caReader) Close(ctx context.Context) error {
	select {
	case <-d.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}
