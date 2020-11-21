package rpc

import (
	"context"
	"encoding/json"
	"fmt"
	"path"
	"reflect"
	"runtime"
	"time"

	"github.com/golang/protobuf/proto"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

var (
	serverRequestCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_rpc_server_request_count",
		Help: "The total number of rpc server requests",
	}, []string{"method"})
	serverErrorCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_rpc_server_error_count",
		Help: "The total number of rpc server errors",
	}, []string{"method"})
	serverRequestDurationMs = promauto.NewHistogramVec(prometheus.HistogramOpts{
		Name: "strims_rpc_server_request_duration_ms",
		Help: "The request duration for rpc server requests",
	}, []string{"method"})
)

// NewServer ...
func NewServer(logger *zap.Logger) *Server {
	return &Server{
		ServiceDispatcher: NewServiceDispatcher(logger),
	}
}

// Server ...
type Server struct {
	*ServiceDispatcher
}

// Listen ...
func (s *Server) Listen(ctx context.Context, dialer Dialer) error {
	transport, err := dialer.Dial(ctx, s.ServiceDispatcher)
	if err != nil {
		return err
	}

	return transport.Listen()
}

// NewServiceDispatcher ...
func NewServiceDispatcher(logger *zap.Logger) *ServiceDispatcher {
	return &ServiceDispatcher{
		logger:  logger,
		methods: map[string]serviceMethod{},
	}
}

// ServiceDispatcher ...
type ServiceDispatcher struct {
	logger  *zap.Logger
	methods map[string]serviceMethod
}

// RegisterMethod ...
func (h *ServiceDispatcher) RegisterMethod(name string, method interface{}) {
	h.methods[name] = serviceMethod{
		fn:                reflect.ValueOf(method),
		requestCount:      serverRequestCount.WithLabelValues(name),
		requestDurationMs: serverRequestDurationMs.WithLabelValues(name),
		errorCount:        serverRequestCount.WithLabelValues(name),
	}
}

// Dispatch ...
func (h *ServiceDispatcher) Dispatch(call *CallIn) {
	switch call.Method() {
	case cancelMethod:
		h.cancel(call)
	default:
		h.call(call)
	}
}

func (h *ServiceDispatcher) cancel(call *CallIn) {
	if parent := call.ParentCallIn(); parent != nil {
		parent.Cancel()
	}
}

func (h *ServiceDispatcher) call(call *CallIn) {
	method, ok := h.methods[call.Method()]
	if !ok {
		call.returnError(fmt.Errorf("method not found: %s", call.Method()))
		return
	}

	method.requestCount.Inc()
	defer func(start time.Time) {
		if err := recoverError(recover()); err != nil {
			method.errorCount.Inc()
			h.logger.Error("call handler panicked", zap.Error(err), zap.Stack("stack"))
			call.returnError(err)
		}

		duration := time.Since(start)
		method.requestDurationMs.Observe(float64(duration / time.Millisecond))
		h.logger.Debug(
			"rpc received",
			zap.String("method", call.Method()),
			zap.Stringer("responseType", call.ResponseType()),
			zap.Duration("duration", duration),
		)
	}(time.Now())

	arg, err := call.Argument()
	if err != nil {
		serverErrorCount.WithLabelValues(call.Method()).Inc()
		call.returnError(err)
		return
	}

	rs := method.fn.Call([]reflect.Value{reflect.ValueOf(call.Context()), reflect.ValueOf(arg)})
	if len(rs) == 0 {
		call.returnUndefined()
	} else if err, ok := rs[len(rs)-1].Interface().(error); ok && err != nil {
		method.errorCount.Inc()
		call.returnError(err)
	} else if r := rs[0]; r.Kind() == reflect.Chan {
		call.returnStream(r.Interface())
	} else if r, ok := rs[0].Interface().(proto.Message); ok {
		call.returnValue(r)
	} else {
		call.returnError(fmt.Errorf("unexpected response type %T", rs[0].Interface()))
	}
}

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}

type serviceMethod struct {
	fn                reflect.Value
	requestCount      prometheus.Counter
	requestDurationMs prometheus.Observer
	errorCount        prometheus.Counter
}
