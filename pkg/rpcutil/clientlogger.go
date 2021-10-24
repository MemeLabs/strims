package rpcutil

import (
	"context"
	"reflect"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

func NewClientLogger(c rpc.Caller, logger *zap.Logger) *ClientLogger {
	return &ClientLogger{
		c:      c,
		logger: logger,
	}
}

type ClientLogger struct {
	c      rpc.Caller
	logger *zap.Logger
}

func (c *ClientLogger) CallUnary(ctx context.Context, method string, req proto.Message, res proto.Message) error {
	start := timeutil.Now()

	err := c.c.CallUnary(ctx, method, req, res)
	if err != nil {
		c.logger.Debug(
			"rpc error",
			zap.Stringer("rtt", timeutil.Now().Sub(start)),
			zap.String("method", method),
			zap.Reflect("req", req),
			zap.Error(err),
		)
		return err
	}

	c.logger.Debug(
		"rpc response",
		zap.Stringer("rtt", timeutil.Now().Sub(start)),
		zap.String("method", method),
		zap.Reflect("req", req),
		zap.Reflect("res", res),
	)
	return nil
}

func (c *ClientLogger) CallStreaming(ctx context.Context, method string, req proto.Message, res interface{}) error {
	errs := make(chan error, 0)
	resIn := reflect.MakeChan(reflect.TypeOf(res), 0)
	resOut := reflect.ValueOf(res)

	go func() {
		errs <- c.c.CallStreaming(ctx, method, req, resIn.Interface())
	}()

	cases := []reflect.SelectCase{
		{
			Dir:  reflect.SelectRecv,
			Chan: resIn,
		},
		{
			Dir:  reflect.SelectRecv,
			Chan: reflect.ValueOf(errs),
		},
	}

	for {
		i, v, _ := reflect.Select(cases)
		switch i {
		case 0:
			c.logger.Debug(
				"rpc stream response",
				zap.String("method", method),
				zap.Reflect("req", req),
				zap.Reflect("res", v.Interface()),
			)
			resOut.Send(v)
		case 1:
			err := v.Interface().(error)
			c.logger.Debug(
				"rpc stream closed",
				zap.String("method", method),
				zap.Reflect("req", req),
				zap.Error(err),
			)
			return err
		}
	}
}
