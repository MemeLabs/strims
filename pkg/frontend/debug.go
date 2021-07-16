package frontend

import (
	"bytes"
	"context"
	"errors"
	"runtime"
	"runtime/pprof"

	debugv1 "github.com/MemeLabs/go-ppspp/pkg/apis/debug/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/common/expfmt"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
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

// ReadMetrics ...
func (s *debugService) ReadMetrics(ctx context.Context, r *debugv1.ReadMetricsRequest) (*debugv1.ReadMetricsResponse, error) {
	var format expfmt.Format
	switch r.Format {
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

	return &debugv1.ReadMetricsResponse{Data: b.Bytes()}, nil
}
