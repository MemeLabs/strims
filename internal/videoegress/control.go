package videoegress

import (
	"context"
	"fmt"
	"io"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// ControlBase ...
type ControlBase interface {
	OpenStream(ctx context.Context, swarmURI string, networkKeys [][]byte) ([]byte, io.ReadCloser, error)
}

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, observers *event.Observers, transfer transfer.Control) Control {
	// events := make(chan interface{}, 8)
	// observers.Notify(events)

	return &control{
		logger:    logger,
		vpn:       vpn,
		observers: observers,
		// events:    events,
		transfer: transfer,
	}
}

// Control ...
type control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	observers *event.Observers
	// events    chan interface{}
	transfer transfer.Control

	lock sync.Mutex
}

// Run ...
func (t *control) Run(ctx context.Context) {
	<-ctx.Done()
	t.close()
}

func (t *control) close() {

}

func (t *control) open(swarmURI string, networkKeys [][]byte) ([]byte, *ppspp.Swarm, bool, error) {
	uri, err := ppspp.ParseURI(swarmURI)
	if err != nil {
		return nil, nil, false, err
	}

	transferID, swarm, ok := t.transfer.Find(uri.ID, nil)
	if ok {
		return transferID, swarm, false, nil
	}

	opt := uri.Options.SwarmOptions()
	opt.LiveWindow = (32 * 1024 * 1024) / opt.ChunkSize

	swarm, err = ppspp.NewSwarm(uri.ID, opt)
	if err != nil {
		return nil, nil, false, err
	}

	transferID = t.transfer.Add(swarm, nil)
	for _, k := range networkKeys {
		t.logger.Debug(
			"publishing transfer",
			logutil.ByteHex("transfer", transferID),
			logutil.ByteHex("network", k),
		)
		t.transfer.Publish(transferID, k)
	}

	return transferID, swarm, true, nil
}

// OpenStream ...
func (t *control) OpenStream(ctx context.Context, swarmURI string, networkKeys [][]byte) ([]byte, io.ReadCloser, error) {
	transferID, swarm, created, err := t.open(swarmURI, networkKeys)
	if err != nil {
		return nil, nil, err
	}

	b := swarm.Reader()
	b.SetReadStopper(ctx.Done())

	r := &VideoReader{
		logger: t.logger.With(
			logutil.ByteHex("transfer", transferID),
			zap.Stringer("swarm", swarm.ID()),
		),
		transfer:   t.transfer,
		transferID: transferID,
		// TODO: removeOnClose should use reference counting
		removeOnClose: created,
		b:             b,
	}
	return transferID, r, nil
}

// VideoReader ...
type VideoReader struct {
	logger        *zap.Logger
	transfer      transfer.Control
	transferID    []byte
	removeOnClose bool
	b             *store.BufferReader
	r             *chunkstream.Reader
}

// Close ...
func (r *VideoReader) Close() error {
	if r.removeOnClose {
		r.transfer.Remove(r.transferID)
	}
	return nil
}

func (r *VideoReader) initReader() (err error) {
	r.r, err = chunkstream.NewReaderSize(r.b, int64(r.b.Offset()), chunkstream.DefaultSize)
	if err != nil {
		return err
	}

	return r.discardFragment()
}

func (r *VideoReader) reinitReader() error {
	if _, err := r.b.Recover(); err != nil {
		return err
	}
	r.r.SetOffset(int64(r.b.Offset()))

	return r.discardFragment()
}

func (r *VideoReader) discardFragment() error {
	off := r.b.Offset()
	r.logger.Debug("discarding segment fragment", zap.Uint64("offset", off))

	for {
		if _, err := io.Copy(io.Discard, r.r); err == nil {
			break
		} else if err != store.ErrBufferUnderrun {
			return err
		}

		if _, err := r.b.Recover(); err != nil {
			return err
		}
		r.r.SetOffset(int64(r.b.Offset()))
	}

	doff := r.b.Offset()
	r.logger.Debug(
		"finished discarding segment fragment",
		zap.Uint64("bytes", doff-off),
		zap.Uint64("offset", doff),
	)

	return nil
}

// Read ...
func (r *VideoReader) Read(b []byte) (int, error) {
	for r.r == nil {
		if err := r.initReader(); err != nil {
			return 0, fmt.Errorf("unable to initialize reader: %w", err)
		}
	}

	n, err := r.r.Read(b)
	if err == store.ErrBufferUnderrun {
		if err := r.reinitReader(); err != nil {
			return 0, err
		}
	}

	return n, err
}
