package ppspptest

import (
	"context"

	"golang.org/x/time/rate"
)

// bit rates
const (
	Gbps = 1 << 30 / 8
	Mbps = 1 << 20 / 8
	Kbps = 1 << 10 / 8
)

// NewConnThrottle ...
func NewConnThrottle(r, w int) *ConnThrottle {
	return &ConnThrottle{
		r: rate.NewLimiter(rate.Limit(r), r),
		w: rate.NewLimiter(rate.Limit(w), w),
	}
}

// ConnThrottle ...
type ConnThrottle struct {
	r, w *rate.Limiter
}

// NewThrottleConn ...
func NewThrottleConn(c Conn, t *ConnThrottle) *ThrottleConn {
	ctx, cancel := context.WithCancel(context.Background())
	return &ThrottleConn{c, t, ctx, cancel}
}

// ThrottleConn ...
type ThrottleConn struct {
	Conn
	t      *ConnThrottle
	ctx    context.Context
	cancel context.CancelFunc
}

// Flush ...
func (c *ThrottleConn) Flush() error {
	if err := c.t.w.WaitN(c.ctx, c.Conn.Buffered()); err != nil {
		return err
	}
	return c.Conn.Flush()
}

// Read ...
func (c *ThrottleConn) Read(p []byte) (int, error) {
	n, err := c.Conn.Read(p)
	if err := c.t.r.WaitN(c.ctx, n); err != nil {
		return 0, err
	}
	return n, err
}

// Close ...
func (c *ThrottleConn) Close() error {
	c.cancel()
	return c.Conn.Close()
}
