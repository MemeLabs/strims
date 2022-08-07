// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package debug

import (
	"bytes"
	"context"
	"strconv"
	"sync"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/transfer"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/chunkstream"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/golang/protobuf/proto"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

var mockStreamLatency = promauto.NewGaugeVec(prometheus.GaugeOpts{
	Name: "strims_debug_mock_stream_latency_ms",
	Help: "The difference in milliseconds between the local clock and received mock stream segment timestamps",
}, []string{"id", "swarm_id"})

type Control interface {
	Run()
	StartMockStream(
		ctx context.Context,
		bitrateBytes int,
		segmentInterval time.Duration,
		timeout time.Duration,
		networkKey []byte,
	) (uint64, error)
	StopMockStream(id uint64)
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	transferControl transfer.Control,
	directoryControl directory.Control,
) Control {
	return &control{
		ctx:       ctx,
		logger:    logger,
		store:     store,
		transfer:  transferControl,
		directory: directoryControl,

		events:             observers.Chan(),
		listingTransferIDs: map[uint64]transfer.ID{},
	}
}

// Control ...
type control struct {
	ctx       context.Context
	logger    *zap.Logger
	store     *dao.ProfileStore
	transfer  transfer.Control
	directory directory.Control

	events             chan any
	lock               sync.Mutex
	config             *debugv1.Config
	listingTransferIDs map[uint64]transfer.ID
	mockStreamClosers  syncutil.Map[uint64, context.CancelFunc]
}

// Run ...
func (c *control) Run() {
	go c.loadConfig()

	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			case *debugv1.ConfigChangeEvent:
				c.handleConfigChange(e.Config)
			case event.DirectoryEvent:
				c.handleDirectoryEvent(e)
			}
		case <-c.ctx.Done():
			return
		}
	}
}

func (c *control) StartMockStream(
	ctx context.Context,
	bitrateBytes int,
	segmentInterval time.Duration,
	timeout time.Duration,
	networkKey []byte,
) (uint64, error) {
	id, err := dao.GenerateSnowflake()
	if err != nil {
		return 0, err
	}

	r := mockStreamRunner{
		ID:              id,
		BitrateBytes:    bitrateBytes,
		SegmentInterval: segmentInterval,
		Timeout:         timeout,
		NetworkKey:      networkKey,
	}

	if err := r.Init(ctx, c.transfer, c.directory); err != nil {
		return 0, err
	}

	ctx, cancel := context.WithCancel(context.Background())
	c.mockStreamClosers.Set(id, cancel)
	go func() {
		if err := r.Run(ctx, c.transfer, c.directory); err != nil {
			c.logger.Debug("mock stream ended", zap.Error(err))
		}
		cancel()
		c.mockStreamClosers.Delete(id)
	}()

	return id, nil
}

func (c *control) StopMockStream(id uint64) {
	cancel, ok := c.mockStreamClosers.Get(id)
	if ok {
		cancel()
	}
}

func (c *control) loadConfig() {
	config, err := dao.DebugConfig.Get(c.store)
	if err != nil {
		c.logger.Warn("failed to load autoseed config", zap.Error(err))
		return
	}

	c.lock.Lock()
	defer c.lock.Unlock()

	c.applyConfig(config)
}

func (c *control) handleConfigChange(config *debugv1.Config) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.applyConfig(config)
}

func (c *control) handleDirectoryEvent(event event.DirectoryEvent) {
	c.lock.Lock()
	defer c.lock.Unlock()

	for _, e := range event.Broadcast.Events {
		switch b := e.Body.(type) {
		case *networkv1directory.Event_ListingChange_:
			c.tryStartSwarm(event.NetworkKey, b.ListingChange.Id, b.ListingChange.Listing)
		case *networkv1directory.Event_Unpublish_:
			c.tryStopSwarm(b.Unpublish.Id)
		}
	}
}

func (c *control) applyConfig(config *debugv1.Config) {
	stopRunning := c.config.GetEnableMockStreams() && !config.EnableMockStreams

	c.config = config

	if stopRunning {
		for lid, tid := range c.listingTransferIDs {
			c.transfer.Remove(tid)
			delete(c.listingTransferIDs, lid)
		}
	}
}

func (c *control) tryStartSwarm(networkKey []byte, listingID uint64, l *networkv1directory.Listing) {
	if !c.config.EnableMockStreams || !bytes.Equal(networkKey, c.config.MockStreamNetworkKey) {
		return
	}

	m := l.GetService()
	if m == nil || m.Type != mockServiceType {
		return
	}
	uri, err := ppspp.ParseURI(m.GetSwarmUri())
	if err != nil {
		return
	}

	if _, _, ok := c.transfer.Find(uri.ID, nil); ok {
		return
	}

	opt := uri.Options.SwarmOptions()
	opt.LiveWindow = (16 * 1024 * 1024) / opt.ChunkSize

	swarm, err := ppspp.NewSwarm(uri.ID, opt)
	if err != nil {
		c.logger.Warn("creating mock stream swarm failed", zap.Error(err))
		return
	}
	c.logger.Debug(
		"created mock stream reader",
		zap.Stringer("swarm", swarm.ID()),
	)

	go c.readSwarm(swarm)

	transferID := c.transfer.Add(swarm, nil)
	c.transfer.Publish(transferID, networkKey)
	c.listingTransferIDs[listingID] = transferID
}

func (c *control) readSwarm(swarm *ppspp.Swarm) {
	r := swarm.Reader()
	cr, err := chunkstream.NewReader(r, int64(r.Offset()))
	if err != nil {
		return
	}

	var latencyGauge prometheus.Gauge
	var buf bytes.Buffer
	for {
		buf.Reset()
		if _, err := buf.ReadFrom(cr); err != nil {
			c.logger.Debug("error reading from mock stream", zap.Error(err))
			return
		}

		var segment debugv1.MockStreamSegment
		if err := proto.Unmarshal(buf.Bytes(), &segment); err != nil {
			c.logger.Debug("error reading from mock stream", zap.Error(err))
			continue
		}

		if latencyGauge == nil {
			labels := []string{strconv.FormatUint(segment.Id, 10), swarm.ID().String()}
			latencyGauge = mockStreamLatency.WithLabelValues(labels...)
			defer mockStreamLatency.DeleteLabelValues(labels...)
		}

		latency := time.Now().Round(time.Duration(time.Microsecond.Seconds())).Sub(time.UnixMicro(segment.Timestamp))
		c.logger.Debug(
			"read mock stream segment",
			zap.Stringer("swarm", swarm.ID()),
			zap.Uint64("id", segment.Id),
			zap.Duration("latency", latency),
		)
		latencyGauge.Set(float64(latency) / float64(time.Microsecond))
	}
}

func (c *control) tryStopSwarm(listingID uint64) {
	tid, ok := c.listingTransferIDs[listingID]
	if ok {
		c.transfer.Remove(tid)
		delete(c.listingTransferIDs, listingID)
	}
}
