package rpc

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"go.uber.org/zap"
)

// RWFDialer ...
type RWFDialer struct {
	Logger           *zap.Logger
	ReadWriteFlusher ioutil.ReadWriteFlusher
}

// Dial ...
func (d *RWFDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	rwd := RWDialer{
		Logger:     d.Logger,
		ReadWriter: rwfWriteFlusher{d.ReadWriteFlusher},
	}
	return rwd.Dial(ctx, dispatcher)
}

type rwfWriteFlusher struct {
	ioutil.ReadWriteFlusher
}

func (r rwfWriteFlusher) Write(p []byte) (int, error) {
	n, err := r.ReadWriteFlusher.Write(p)
	if err != nil {
		return n, err
	}
	return n, r.ReadWriteFlusher.Flush()
}
