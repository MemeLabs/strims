package rpc

import (
	"context"
	"io"

	"go.uber.org/zap"
)

// RWFDialer ...
type RWFDialer struct {
	Logger           *zap.Logger
	ReadWriteFlusher ReadWriteFlusher
}

// Dial ...
func (d *RWFDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	rwd := RWDialer{
		Logger:     d.Logger,
		ReadWriter: rwfWriteFlusher{d.ReadWriteFlusher},
	}
	return rwd.Dial(ctx, dispatcher)
}

// ReadWriteFlusher ...
type ReadWriteFlusher interface {
	io.ReadWriter
	Flush() error
}

type rwfWriteFlusher struct {
	ReadWriteFlusher
}

func (r rwfWriteFlusher) Write(p []byte) (int, error) {
	n, err := r.ReadWriteFlusher.Write(p)
	if err != nil {
		return n, err
	}
	return n, r.ReadWriteFlusher.Flush()
}
