package ppspptest

import (
	"math/rand"
	"time"

	"github.com/golang/geo/s2"
)

const (
	earthRadius = 6378.1370
	c           = 300000
	linkSpeed   = c / 3
)

// ComputeLatency ...
func ComputeLatency(a, b s2.LatLng) time.Duration {
	d := a.Distance(b).Radians() * earthRadius
	return time.Duration(float64(time.Second) * d / linkSpeed)
}

// NewLagConnPair ...
func NewLagConnPair(a, b Conn, l time.Duration, jitter float64) (*LagConn, *LagConn) {
	ach := make(chan time.Time, 128)
	bch := make(chan time.Time, 128)
	j := int64(float64(l) * jitter)
	return &LagConn{a, l, j, ach, bch}, &LagConn{b, l, j, bch, ach}
}

// LagConn ...
type LagConn struct {
	Conn
	latency time.Duration
	jitter  int64
	wch     chan time.Time
	rch     <-chan time.Time
}

// Flush ...
func (c *LagConn) Flush() error {
	if err := c.Conn.Flush(); err != nil {
		return err
	}

	l := c.latency
	if c.jitter > 0 {
		l += time.Duration(rand.Int63n(c.jitter))
	}
	c.wch <- time.Now().Add(l)
	return nil
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
