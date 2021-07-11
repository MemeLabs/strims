package ppspptest

import (
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/stats"
)

// NewMeterConn ...
func NewMeterConn(c Conn) *MeterConn {
	return &MeterConn{
		Conn: c,
		rma:  stats.NewSMA(100, 10*time.Millisecond),
		wma:  stats.NewSMA(100, 10*time.Millisecond),
	}
}

// MeterConn ...
type MeterConn struct {
	Conn
	mu       sync.Mutex
	rn, wn   int64
	rma, wma stats.SMA
}

// ReadBytes ...
func (c *MeterConn) ReadBytes() int64 {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.rn
}

// WrittenBytes ...
func (c *MeterConn) WrittenBytes() int64 {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.wn
}

// ReadByteRate ...
func (c *MeterConn) ReadByteRate() int64 {
	c.mu.Lock()
	defer c.mu.Unlock()
	return int64(c.rma.Rate(time.Second))
}

// WriteByteRate ...
func (c *MeterConn) WriteByteRate() int64 {
	c.mu.Lock()
	defer c.mu.Unlock()
	return int64(c.wma.Rate(time.Second))
}

// Read ...
func (c *MeterConn) Read(p []byte) (int, error) {
	n, err := c.Conn.Read(p)

	c.mu.Lock()
	defer c.mu.Unlock()
	c.rn += int64(n)
	c.rma.Add(uint64(n))

	return n, err
}

// Flush ...
func (c *MeterConn) Flush() error {
	n := c.Conn.Buffered()
	err := c.Conn.Flush()

	c.mu.Lock()
	defer c.mu.Unlock()
	c.wn += int64(n)
	c.wma.Add(uint64(n))

	return err
}
