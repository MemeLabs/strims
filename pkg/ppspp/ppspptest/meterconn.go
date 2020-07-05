package ppspptest

import "sync/atomic"

// NewMeterConn ...
func NewMeterConn(c Conn) *MeterConn {
	return &MeterConn{Conn: c}
}

// MeterConn ...
type MeterConn struct {
	Conn
	rn, wn int64
}

// ReadBytes ...
func (c *MeterConn) ReadBytes() int64 {
	return atomic.LoadInt64(&c.rn)
}

// WrittenBytes ...
func (c *MeterConn) WrittenBytes() int64 {
	return atomic.LoadInt64(&c.wn)
}

// Read ...
func (c *MeterConn) Read(p []byte) (int, error) {
	n, err := c.Conn.Read(p)
	atomic.AddInt64(&c.rn, int64(n))
	return n, err
}

// Write ...
func (c *MeterConn) Write(p []byte) (int, error) {
	n, err := c.Conn.Write(p)
	atomic.AddInt64(&c.wn, int64(n))
	return n, err
}
