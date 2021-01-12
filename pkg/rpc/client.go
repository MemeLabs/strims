package rpc

import (
	"context"
	"errors"
	"runtime"
	"time"

	rpcv1 "github.com/MemeLabs/go-ppspp/pkg/apis/rpc/v1"
	"github.com/golang/protobuf/proto"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

var (
	clientRequestCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_rpc_client_request_count",
		Help: "The total number of rpc client requests",
	}, []string{"method"})
	clientErrorCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_rpc_client_error_count",
		Help: "The total number of rpc client errors",
	}, []string{"method"})
	clientRequestDurationMs = promauto.NewHistogramVec(prometheus.HistogramOpts{
		Name: "strims_rpc_client_request_duration_ms",
		Help: "The request duration for rpc client requests",
	}, []string{"method"})
)

// NewClient ...
func NewClient(logger *zap.Logger, dialer Dialer) (*Client, error) {
	ctx, cancel := context.WithCancel(context.Background())

	transport, err := dialer.Dial(ctx, &clientDispatcher{})
	if err != nil {
		cancel()
		return nil, err
	}

	ready := make(chan struct{})
	go func() {
		close(ready)
		if err := transport.Listen(); err != nil && !errors.Is(err, context.Canceled) {
			logger.Debug("client closed with error", zap.Error(err))
		}
		cancel()
	}()
	<-ready

	c := &Client{
		logger:    logger,
		transport: transport,
		cancel:    cancel,
	}

	runtime.SetFinalizer(c, clientFinalizer)

	return c, nil
}

func clientFinalizer(c *Client) {
	c.Close()
}

// Client ...
type Client struct {
	logger    *zap.Logger
	transport Transport
	cancel    context.CancelFunc
}

// Close ...
func (c *Client) Close() {
	c.cancel()
}

func (c *Client) forwardCancel(call *CallOut) error {
	call, err := NewCallOutWithParent(context.Background(), cancelMethod, &rpcv1.Cancel{}, call)
	if err != nil {
		return err
	}
	return c.transport.Call(call, func() error { return nil })
}

func (c *Client) handleError(call *CallOut, err error) error {
	if err != nil {
		clientErrorCount.WithLabelValues(call.Method()).Inc()
	}

	if errors.Is(err, context.Canceled) || errors.Is(err, context.DeadlineExceeded) {
		if err := c.forwardCancel(call); err != nil {
			c.logger.Debug("forwarding request cancellation failed", zap.Error(err))
		}
	}
	return err
}

// CallUnary ...
func (c *Client) CallUnary(ctx context.Context, method string, req, res proto.Message) error {
	clientRequestCount.WithLabelValues(method).Inc()
	defer func(start time.Time) {
		duration := time.Since(start) / time.Millisecond
		clientRequestDurationMs.WithLabelValues(method).Observe(float64(duration))
	}(time.Now())

	call, err := NewCallOut(ctx, method, req)
	if err != nil {
		return err
	}

	return c.transport.Call(call, func() error {
		return c.handleError(call, call.ReadResponse(res))
	})
}

// CallStreaming ...
func (c *Client) CallStreaming(ctx context.Context, method string, req proto.Message, res interface{}) error {
	clientRequestCount.WithLabelValues(method).Inc()

	call, err := NewCallOut(ctx, method, req)
	if err != nil {
		return err
	}

	return c.transport.Call(call, func() error {
		return c.handleError(call, call.ReadResponseStream(res))
	})
}

type clientDispatcher struct{}

func (c *clientDispatcher) Dispatch(call *CallIn) {
	if call.Method() != callbackMethod {
		return
	}

	parent := call.ParentCallOut()
	if parent == nil {
		return
	}

	parent.AssignResponse(call)
}
