package rpcutil

import (
	"context"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"google.golang.org/protobuf/proto"
)

func NewClientRetrier(c rpc.Caller, maxRetries, backoff int, delay, timeout time.Duration) *ClientRetrier {
	return &ClientRetrier{
		c:          c,
		maxRetries: maxRetries,
		backoff:    backoff,
		delay:      delay,
		timeout:    timeout,
	}
}

type ClientRetrier struct {
	c          rpc.Caller
	maxRetries int
	backoff    int
	delay      time.Duration
	timeout    time.Duration
}

func (c *ClientRetrier) CallUnary(ctx context.Context, method string, req proto.Message, res proto.Message) error {
	retries := c.maxRetries
	delay := c.delay

	for {
		callCtx, cancel := context.WithTimeout(ctx, c.timeout)
		err := c.c.CallUnary(callCtx, method, req, res)
		cancel()

		if err == nil || retries <= 0 || ctx.Err() == context.Canceled || ctx.Err() == context.DeadlineExceeded {
			return err
		}

		time.Sleep(delay)

		retries--
		if c.backoff > 0 {
			delay *= time.Duration(c.backoff)
		}
	}
}

func (c *ClientRetrier) CallStreaming(ctx context.Context, method string, req proto.Message, res interface{}) error {
	return c.c.CallStreaming(ctx, method, req, res)
}
