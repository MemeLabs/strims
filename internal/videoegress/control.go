package videoegress

import (
	"bytes"
	"context"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/transfer"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/hls"
	"github.com/MemeLabs/go-ppspp/pkg/httputil"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"go.uber.org/zap"
)

// Control ...
type Control interface {
	Run()
	OpenStream(ctx context.Context, swarmURI string, networkKeys [][]byte) (transfer.ID, io.ReadCloser, error)
	OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error)
	CloseHLSStream(swarmURI string) error
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	httpmux *httputil.MapServeMux,
	profile *profilev1.Profile,
	transfer transfer.Control,
) Control {
	hlsPath := fmt.Sprintf("/hls/%x", profile.Key.Public)

	return &control{
		ctx:      ctx,
		logger:   logger,
		httpmux:  httpmux,
		transfer: transfer,

		hlsPath:    hlsPath,
		hlsService: hls.NewService(hlsPath),
	}
}

// Control ...
type control struct {
	ctx      context.Context
	logger   *zap.Logger
	httpmux  *httputil.MapServeMux
	transfer transfer.Control

	lock       sync.Mutex
	hlsPath    string
	hlsService *hls.Service
}

// Run ...
func (t *control) Run() {
	path := t.hlsPath + "/*"

	t.logger.Debug("registering hls service", zap.String("path", path))
	t.httpmux.Handle(path, t.hlsService.Handler())

	<-t.ctx.Done()

	t.logger.Debug("removing hls service", zap.String("path", path))
	t.httpmux.StopHandling(path)
}

func (t *control) open(swarmURI string, networkKeys [][]byte) (transfer.ID, *ppspp.Swarm, bool, error) {
	uri, err := ppspp.ParseURI(swarmURI)
	if err != nil {
		return transfer.ID{}, nil, false, err
	}

	transferID, swarm, ok := t.transfer.Find(uri.ID, nil)
	if ok {
		return transferID, swarm, false, nil
	}

	opt := uri.Options.SwarmOptions()
	opt.LiveWindow = (32 * 1024 * 1024) / opt.ChunkSize

	swarm, err = ppspp.NewSwarm(uri.ID, opt)
	if err != nil {
		return transfer.ID{}, nil, false, err
	}

	transferID = t.transfer.Add(swarm, nil)
	for _, k := range networkKeys {
		t.logger.Debug(
			"publishing transfer",
			logutil.ByteHex("transfer", transferID[:]),
			logutil.ByteHex("network", k),
		)
		t.transfer.Publish(transferID, k)
	}

	return transferID, swarm, true, nil
}

// OpenStream ...
func (t *control) OpenStream(ctx context.Context, swarmURI string, networkKeys [][]byte) (transfer.ID, io.ReadCloser, error) {
	transferID, swarm, created, err := t.open(swarmURI, networkKeys)
	if err != nil {
		return transfer.ID{}, nil, err
	}

	b := swarm.Reader()
	b.SetReadStopper(ctx.Done())

	r := &VideoReader{
		logger: t.logger.With(
			logutil.ByteHex("transfer", transferID[:]),
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

// OpenHLSStream ...
func (t *control) OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error) {
	transferID, swarm, created, err := t.open(swarmURI, networkKeys)
	if err != nil {
		return "", err
	}

	// TODO: prevent opening more than once

	c := &hls.Channel{
		Name:   hex.EncodeToString(transferID[:]),
		Stream: hls.NewDefaultStream(),
	}
	uri := t.hlsService.InsertChannel(c)

	r := &VideoReader{
		logger: t.logger.With(
			logutil.ByteHex("transfer", transferID[:]),
			zap.Stringer("swarm", swarm.ID()),
		),
		transfer:      t.transfer,
		transferID:    transferID,
		removeOnClose: created,
		b:             swarm.Reader(),
	}

	r.logger.Debug("hls stream starting", zap.String("uri", uri))

	go func() {
		defer t.hlsService.RemoveChannel(c)
		defer r.Close()

		var initWritten bool

		var buf bytes.Buffer
		for {
			buf.Reset()

			_, err := io.Copy(&buf, r)
			switch err {
			case nil:
				init := buf.Next(int(binary.BigEndian.Uint16(buf.Next(2))))
				if !initWritten {
					initWritten = true
					w := c.Stream.InitWriter()
					w.Write(init)
					w.Close()
				}

				w := c.Stream.NextWriter()
				buf.WriteTo(w)
				w.Close()
			case store.ErrBufferUnderrun:
				c.Stream.MarkDiscontinuity()
			default:
				t.logger.Debug("stream closed", zap.Error(err))
				return
			}
		}
	}()

	return uri, nil
}

// CloseHLSStream ...
func (t *control) CloseHLSStream(swarmURI string) error {
	return nil
}

// VideoReader ...
type VideoReader struct {
	logger        *zap.Logger
	transfer      transfer.Control
	transferID    transfer.ID
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
