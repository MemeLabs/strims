package directory

import (
	"context"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
)

func newDirectoryReader(logger *zap.Logger, key []byte) (*directoryReader, error) {
	s, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), swarmOptions)
	if err != nil {
		return nil, err
	}

	return &directoryReader{
		logger:      logger,
		key:         key,
		swarm:       s,
		eventReader: protoutil.NewChunkStreamReader(s.Reader(), chunkSize),
		close:       make(chan struct{}, 1),
	}, nil
}

type directoryReader struct {
	logger      *zap.Logger
	key         []byte
	swarm       *ppspp.Swarm
	eventReader *protoutil.ChunkStreamReader
	cancel      context.CancelFunc
	close       chan struct{}
}

func (d *directoryReader) Run(ctx context.Context, transfer control.TransferControl) error {
	ctx, cancel := context.WithCancel(ctx)
	d.cancel = cancel

	transferID := transfer.Add(d.swarm, AddressSalt)
	transfer.Publish(transferID, d.key)

	select {
	case <-ctx.Done():
	case <-d.close:
		d.cancel()
	}

	transfer.Remove(transferID)
	d.swarm.Close()

	d.cancel = nil

	return ctx.Err()
}

func (d *directoryReader) Close() {
	if d == nil {
		return
	}
	d.close <- struct{}{}

	if d.cancel == nil {
		return
	}
	d.cancel()
}
