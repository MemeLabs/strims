// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package videoegress

import (
	"bytes"
	"context"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/transfer"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
	"github.com/MemeLabs/strims/pkg/chunkstream"
	"github.com/MemeLabs/strims/pkg/hls"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
	"go.uber.org/zap"
)

// Control ...
type Control interface {
	Run()
	OpenStream(ctx context.Context, swarmURI string, networkKeys [][]byte) (transfer.ID, io.ReadCloser, error)
	HLSEgressEnabled() bool
	OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error)
	CloseHLSStream(swarmURI string) error
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	httpmux *httputil.MapServeMux,
	profile *profilev1.Profile,
	transfer transfer.Control,
) Control {
	return &control{
		ctx:      ctx,
		logger:   logger,
		store:    store,
		httpmux:  httpmux,
		transfer: transfer,

		events:      observers.Chan(),
		routePrefix: fmt.Sprintf("/hls/%x", profile.Key.Public),
	}
}

// Control ...
type control struct {
	ctx      context.Context
	logger   *zap.Logger
	store    *dao.ProfileStore
	httpmux  *httputil.MapServeMux
	transfer transfer.Control
	profile  *profilev1.Profile

	events      chan any
	routePrefix string
	mu          sync.Mutex
	config      *videov1.HLSEgressConfig
	service     *hls.Service
	stop        chan struct{}
}

// Run ...
func (t *control) Run() {
	go t.loadConfig()

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case *videov1.HLSEgressConfigChangeEvent:
				t.handleConfigChange(e.EgressConfig)
			}
		case <-t.ctx.Done():
			t.stopService()
			return
		}
	}
}

func (t *control) loadConfig() {
	t.mu.Lock()
	defer t.mu.Unlock()

	config, err := dao.HLSEgressConfig.Get(t.store)
	if err != nil {
		t.logger.Debug("error loading hls egress config", zap.Error(err))
		return
	}
	t.syncConfig(config)
}

func (t *control) handleConfigChange(config *videov1.HLSEgressConfig) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.syncConfig(config)
}

func (t *control) syncConfig(config *videov1.HLSEgressConfig) {
	t.config = config

	if config.Enabled {
		t.startService(config)
	} else {
		t.stopService()
	}
}

func (t *control) startService(config *videov1.HLSEgressConfig) {
	t.logger.Debug("starting hls egress", zap.String("path", t.routePrefix))

	t.service = hls.NewService(t.routePrefix)
	t.httpmux.Handle(t.routePrefix+"/*", t.service.Handler())
	t.stop = make(chan struct{})
}

func (t *control) stopService() {
	if t.service != nil {
		t.logger.Debug("stopping hls egress")

		t.service = nil
		t.httpmux.StopHandling(t.routePrefix + "/*")
		close(t.stop)
	}
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

func (t *control) HLSEgressEnabled() bool {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.config != nil && t.config.Enabled
}

// OpenHLSStream ...
func (t *control) OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error) {
	transferID, swarm, created, err := t.open(swarmURI, networkKeys)
	if err != nil {
		return "", err
	}

	t.mu.Lock()
	config := t.config
	svc := t.service
	t.mu.Unlock()

	if config != nil && svc == nil {
		return "", errors.New("cannot add stream while hls service stopped")
	}

	name := hex.EncodeToString(transferID[:])
	stream := hls.NewDefaultStream()
	uri := config.PublicServerAddr + svc.PlaylistRoute(name)

	if !svc.InsertStream(name, stream) {
		return uri, nil
	}

	b := swarm.Reader()
	b.SetReadStopper(t.stop)

	r := &VideoReader{
		logger: t.logger.With(
			logutil.ByteHex("transfer", transferID[:]),
			zap.Stringer("swarm", swarm.ID()),
		),
		transfer:      t.transfer,
		transferID:    transferID,
		removeOnClose: created,
		b:             b,
	}

	r.logger.Debug("hls stream starting", zap.String("uri", uri))

	go func() {
		defer svc.RemoveStream(name)
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
					w := stream.InitWriter()
					w.Write(init)
					w.Close()
				}

				w := stream.NextWriter()
				buf.WriteTo(w)
				w.Close()
			case store.ErrStreamReset:
				fallthrough
			case store.ErrBufferUnderrun:
				stream.MarkDiscontinuity()
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

EachChunk:
	for {
		_, err := io.Copy(io.Discard, r.r)
		switch err {
		case nil:
			break EachChunk
		case store.ErrBufferUnderrun:
			if _, err := r.b.Recover(); err != nil {
				return err
			}
			fallthrough
		case store.ErrStreamReset:
			r.r.SetOffset(int64(r.b.Offset()))
		default:
			return err
		}
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
	switch err {
	case store.ErrBufferUnderrun:
		if _, err := r.b.Recover(); err != nil {
			return 0, err
		}
		fallthrough
	case store.ErrStreamReset:
		r.r.SetOffset(int64(r.b.Offset()))
		if err := r.discardFragment(); err != nil {
			return 0, err
		}
	}
	return n, err
}
