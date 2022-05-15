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
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/common/expfmt"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		debugv1.RegisterDebugService(server, &debugService{})
	})
}

// debugService ...
type debugService struct{}

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
