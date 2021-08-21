package chat

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
)

func newChatReader(logger *zap.Logger, key, networkKey []byte) (*chatReader, error) {
	s, err := ppspp.NewSwarm(ppspp.NewSwarmID(key), eventSwarmOptions)
	if err != nil {
		return nil, err
	}

	return &chatReader{
		logger:      logger,
		key:         key,
		networkKey:  networkKey,
		swarm:       s,
		eventReader: protoutil.NewChunkStreamReader(s.Reader(), eventChunkSize),
	}, nil
}

type chatReader struct {
	logger      *zap.Logger
	key         []byte
	networkKey  []byte
	swarm       *ppspp.Swarm
	eventReader *protoutil.ChunkStreamReader
	cancel      context.CancelFunc
}

func (d *chatReader) Run(ctx context.Context, transfer control.TransferControl) error {
	ctx, cancel := context.WithCancel(ctx)
	d.cancel = cancel

	transferID := transfer.Add(d.swarm, EventsAddressSalt)
	transfer.Publish(transferID, d.networkKey)

	<-ctx.Done()

	transfer.Remove(transferID)
	d.swarm.Close()

	d.cancel = nil

	return ctx.Err()
}

func (d *chatReader) Close() {
	if d == nil || d.cancel == nil {
		return
	}
	d.cancel()
}
