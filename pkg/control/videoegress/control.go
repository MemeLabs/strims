package videoegress

import (
	"bytes"
	"context"
	"io"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, observers *event.Observers, transfer *transfer.Control) *Control {
	events := make(chan interface{}, 128)
	observers.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		observers: observers,
		events:    events,
		transfer:  transfer,
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	observers *event.Observers
	events    chan interface{}
	transfer  *transfer.Control

	lock sync.Mutex
}

// Run ...
func (t *Control) Run(ctx context.Context) {
	<-ctx.Done()
	t.close()
}

func (t *Control) close() {

}

func (t *Control) open(swarmURI string) ([]byte, *ppspp.Swarm, error) {
	uri, err := ppspp.ParseURI(swarmURI)
	if err != nil {
		return nil, nil, err
	}

	opt := uri.Options.SwarmOptions()
	opt.LiveWindow = (32 * 1024 * 1024) / opt.ChunkSize

	swarm, err := ppspp.NewSwarm(uri.ID, opt)
	if err != nil {
		return nil, nil, err
	}

	transferID := t.transfer.Add(swarm, []byte{})

	return transferID, swarm, nil
}

// OpenStream ...
func (t *Control) OpenStream(swarmURI string) ([]byte, io.ReadCloser, error) {
	transferID, swarm, err := t.open(swarmURI)
	if err != nil {
		return nil, nil, err
	}

	t.logger.Debug("finished discarding chunk fragment")

	r := &VideoReader{
		logger:     t.logger,
		transfer:   t.transfer,
		transferID: transferID,
		swarm:      swarm,
	}
	r.initReader()

	return transferID, r, nil
}

// VideoReader ...
type VideoReader struct {
	logger     *zap.Logger
	transfer   *transfer.Control
	transferID []byte
	swarm      *ppspp.Swarm
	r          io.Reader
}

// Close ...
func (r *VideoReader) Close() error {
	r.transfer.Remove(r.transferID)
	return nil
}

func (r *VideoReader) reinitReader() error {
	sr := r.swarm.Reader()
	sr.SetOffset(sr.Bins().FindLastFilled().BaseLeft())

	return r.initReader()
}

func (r *VideoReader) initReader() error {
	sr := r.swarm.Reader()
	r.logger.Debug("got swarm reader", zap.Uint64("offset", sr.Offset()))
	cr, err := chunkstream.NewReaderSize(sr, int64(sr.Offset()), chunkstream.MaxSize)
	if err != nil {
		return err
	}
	r.logger.Debug("opened chunkstream reader")

	// discard first fragment
	var b bytes.Buffer
	if _, err := io.Copy(&b, cr); err != nil {
		return err
	}

	r.r = cr
	return nil
}

// Read ...
func (r *VideoReader) Read(b []byte) (int, error) {
	n, err := r.r.Read(b)
	if err == store.ErrBufferUnderrun {
		r.reinitReader()
	}

	return n, err
}
