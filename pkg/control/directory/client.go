package directory

import (
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"go.uber.org/zap"
)

func newDirectoryClient(logger *zap.Logger, key []byte, transfer *transfer.Control) (*directoryClient, error) {
	s, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), swarmOptions)
	if err != nil {
		return nil, err
	}

	transferID := transfer.Add(s, AddressSalt)
	transfer.Publish(transferID, key)

	return &directoryClient{
		logger:      logger,
		transfer:    transfer,
		done:        make(chan struct{}),
		swarm:       s,
		transferID:  transferID,
		eventReader: newEventReader(s.Reader()),
	}, nil
}

type directoryClient struct {
	logger     *zap.Logger
	transfer   *transfer.Control
	closeOnce  sync.Once
	done       chan struct{}
	swarm      *ppspp.Swarm
	transferID []byte
	*eventReader
}

func (d *directoryClient) Run(ctx context.Context) error {
	defer d.Close()

	select {
	case <-d.done:
		return errors.New("closed")
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (d *directoryClient) Close() {
	d.closeOnce.Do(func() {
		d.transfer.Remove(d.transferID)
		close(d.done)
	})
}

func (d *directoryClient) ping() error {
	return nil
}
