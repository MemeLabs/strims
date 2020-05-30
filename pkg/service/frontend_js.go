// +build js,wasm

package service

import (
	"bytes"
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/common/expfmt"
)

// ReadMetrics ...
func (s *Frontend) ReadMetrics(ctx context.Context, r *pb.ReadMetricsRequest) (*pb.ReadMetricsResponse, error) {
	var format expfmt.Format
	switch r.Format {
	case pb.MetricsFormat_METRICS_FORMAT_TEXT:
		format = expfmt.FmtText
	case pb.MetricsFormat_METRICS_FORMAT_PROTO_DELIM:
		format = expfmt.FmtProtoDelim
	case pb.MetricsFormat_METRICS_FORMAT_PROTO_TEXT:
		format = expfmt.FmtProtoText
	case pb.MetricsFormat_METRICS_FORMAT_PROTO_COMPACT:
		format = expfmt.FmtProtoCompact
	case pb.MetricsFormat_METRICS_FORMAT_OPEN_METRICS:
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

	return &pb.ReadMetricsResponse{Data: b.Bytes()}, nil
}
