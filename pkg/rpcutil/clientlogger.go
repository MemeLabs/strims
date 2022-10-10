// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package rpcutil

import (
	"context"
	"reflect"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
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
			zap.Stringer("duration", timeutil.Now().Sub(start)),
			zap.String("method", method),
			logutil.Proto("req", req),
			zap.Error(err),
		)
		return err
	}

	c.logger.Debug(
		"rpc response",
		zap.Stringer("duration", timeutil.Now().Sub(start)),
		zap.String("method", method),
		logutil.Proto("req", req),
		logutil.Proto("res", res),
	)
	return nil
}

func (c *ClientLogger) CallStreaming(ctx context.Context, method string, req proto.Message, res any) error {
	errs := make(chan error, 0)
	resIn := reflect.MakeChan(reflect.TypeOf(res), 0)
	resOut := reflect.ValueOf(res)

	logger := c.logger.With(
		zap.String("method", method),
		logutil.Proto("req", req),
	)

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
		i, v, ok := reflect.Select(cases)
		if !ok {
			logger.Debug("rpc stream closed")
			resOut.Close()
			return nil
		}

		switch i {
		case 0:
			logger.Debug("rpc stream response", logutil.Proto("res", v.Interface().(proto.Message)))
			resOut.Send(v)
		case 1:
			err := v.Interface().(error)
			logger.Debug("rpc stream closed", zap.Error(err))
			return err
		}
	}
}
