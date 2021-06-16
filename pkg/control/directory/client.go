package directory

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
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
		eventReader: newEventReader(s.Reader()),
	}, nil
}

type directoryReader struct {
	logger      *zap.Logger
	key         []byte
	swarm       *ppspp.Swarm
	eventReader *EventReader
	cancel      context.CancelFunc
}

func (d *directoryReader) Run(ctx context.Context, transfer *transfer.Control) error {
	ctx, cancel := context.WithCancel(ctx)
	d.cancel = cancel

	transferID := transfer.Add(d.swarm, AddressSalt)
	transfer.Publish(transferID, d.key)

	<-ctx.Done()

	transfer.Remove(transferID)
	d.swarm.Close()

	d.cancel = nil

	return ctx.Err()
}

func (d *directoryReader) Close() {
	if d == nil || d.cancel == nil {
		return
	}
	d.cancel()
}
