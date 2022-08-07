// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"bytes"
	"context"
	"errors"
	"runtime"
	"runtime/pprof"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/common/expfmt"
	"go.uber.org/zap"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		debugv1.RegisterDebugService(server, &debugService{
			store:  params.Store,
			app:    params.App,
			logger: params.Logger,
		})
	})
}

// debugService ...
type debugService struct {
	debugv1.UnimplementedDebugService

	store  *dao.ProfileStore
	app    app.Control
	logger *zap.Logger

	nextMockStreamID uint64
	mockStreams      syncutil.Map[uint64, context.CancelFunc]
}

// PProf ...
func (s *debugService) PProf(ctx context.Context, r *debugv1.PProfRequest) (*debugv1.PProfResponse, error) {
	p := pprof.Lookup(r.Name)
	if p == nil {
		return nil, errors.New("unknown profile")
	}
	if r.Name == "heap" && r.Gc {
		runtime.GC()
	}

	b := &bytes.Buffer{}

	var dbg int
	if r.Debug {
		dbg = 1
	}
	if err := p.WriteTo(b, dbg); err != nil {
		return nil, err
	}

	return &debugv1.PProfResponse{Name: r.Name, Data: b.Bytes()}, nil
}

func (s *debugService) gatherMetrics(f debugv1.MetricsFormat) ([]byte, error) {
	var format expfmt.Format
	switch f {
	case debugv1.MetricsFormat_METRICS_FORMAT_TEXT:
		format = expfmt.FmtText
	case debugv1.MetricsFormat_METRICS_FORMAT_PROTO_DELIM:
		format = expfmt.FmtProtoDelim
	case debugv1.MetricsFormat_METRICS_FORMAT_PROTO_TEXT:
		format = expfmt.FmtProtoText
	case debugv1.MetricsFormat_METRICS_FORMAT_PROTO_COMPACT:
		format = expfmt.FmtProtoCompact
	case debugv1.MetricsFormat_METRICS_FORMAT_OPEN_METRICS:
		format = expfmt.FmtOpenMetrics
	default:
		return nil, errors.New("invalid format")
	}

	mfs, err := prometheus.DefaultGatherer.Gather()
	if err != nil {
		return nil, err
	}

	var b bytes.Buffer
	enc := expfmt.NewEncoder(&b, format)
	for _, mf := range mfs {
		if err := enc.Encode(mf); err != nil {
			return nil, err
		}
	}

	return b.Bytes(), nil
}

// ReadMetrics ...
func (s *debugService) ReadMetrics(ctx context.Context, r *debugv1.ReadMetricsRequest) (*debugv1.ReadMetricsResponse, error) {
	b, err := s.gatherMetrics(r.Format)
	if err != nil {
		return nil, err
	}
	return &debugv1.ReadMetricsResponse{Data: b}, err
}

// WatchMetrics ...
func (s *debugService) WatchMetrics(ctx context.Context, r *debugv1.WatchMetricsRequest) (<-chan *debugv1.WatchMetricsResponse, error) {
	ch := make(chan *debugv1.WatchMetricsResponse)
	go func() {
		defer close(ch)

		t := timeutil.DefaultTickEmitter.Ticker(time.Duration(r.IntervalMs) * time.Millisecond)
		defer t.Stop()

		for {
			select {
			case <-t.C:
				b, err := s.gatherMetrics(r.Format)
				if err != nil {
					return
				}
				ch <- &debugv1.WatchMetricsResponse{Data: b}
			case <-ctx.Done():
				return
			}
		}
	}()
	return ch, nil
}

func (s *debugService) GetConfig(ctx context.Context, r *debugv1.GetConfigRequest) (*debugv1.GetConfigResponse, error) {
	config, err := dao.DebugConfig.Get(s.store)
	if err != nil {
		return nil, err
	}
	return &debugv1.GetConfigResponse{Config: config}, nil
}

func (s *debugService) SetConfig(ctx context.Context, r *debugv1.SetConfigRequest) (*debugv1.SetConfigResponse, error) {
	if err := dao.DebugConfig.Set(s.store, r.Config); err != nil {
		return nil, err
	}
	return &debugv1.SetConfigResponse{Config: r.Config}, nil
}

// StartMockStream ...
func (s *debugService) StartMockStream(ctx context.Context, r *debugv1.StartMockStreamRequest) (*debugv1.StartMockStreamResponse, error) {
	bitrateBytes := int(r.BitrateKbps) * 1024 / 8

	segmentInterval := time.Second
	if r.SegmentIntervalMs != 0 {
		segmentInterval = time.Duration(r.SegmentIntervalMs) * time.Millisecond
	}

	var timeout time.Duration
	if r.TimeoutMs != 0 {
		timeout = time.Duration(r.TimeoutMs) * time.Millisecond
	}

	id, err := s.app.Debug().StartMockStream(ctx, bitrateBytes, segmentInterval, timeout, r.NetworkKey)
	if err != nil {
		return nil, err
	}

	return &debugv1.StartMockStreamResponse{Id: id}, nil
}

// StopMockStream ...
func (s *debugService) StopMockStream(ctx context.Context, r *debugv1.StopMockStreamRequest) (*debugv1.StopMockStreamResponse, error) {
	s.app.Debug().StopMockStream(r.Id)
	return &debugv1.StopMockStreamResponse{}, nil
}
