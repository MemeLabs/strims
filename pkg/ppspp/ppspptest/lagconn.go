package ppspptest

import (
	"time"
)

// NewLagConnPair ...
func NewLagConnPair(a, b Conn, l time.Duration) (*LagConn, *LagConn) {
	ach := make(chan time.Time, 128)
	bch := make(chan time.Time, 128)
	return &LagConn{a, l, ach, bch}, &LagConn{b, l, bch, ach}
}

// LagConn ...
type LagConn struct {
	Conn
	latency time.Duration
	wch     chan time.Time
	rch     <-chan time.Time
}

// Flush ...
func (c *LagConn) Flush() error {
	err := c.Conn.Flush()
	c.wch <- time.Now().Add(c.latency)
	return err
}

// Read ...
func (c *LagConn) Read(p []byte) (int, error) {
	t := <-c.rch
	now := time.Now()
	if t.After(now) {
		time.Sleep(t.Sub(now))
	}

	return c.Conn.Read(p)
}
