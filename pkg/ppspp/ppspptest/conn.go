package ppspptest

import (
	"sync"
	"sync/atomic"
)

const connMTU = 1 << 14

// Conn ...
type Conn interface {
	Write(p []byte) (int, error)
	Flush() error
	Buffered() int
	Close() error
	MTU() int
	Read(p []byte) (int, error)
}

// NewConnPair ...
func NewConnPair() (Conn, Conn) {
	ar, aw := newBufPipe()
	br, bw := newBufPipe()

	return &conn{ar, bw}, &conn{br, aw}
}

// Conn ...
type conn struct {
	r *bufPipeReader
	w *bufPipeWriter
}

// Write ...
func (c *conn) Write(p []byte) (int, error) {
	return c.w.Write(p)
}

// Flush ...
func (c *conn) Flush() error {
	return c.w.Flush()
}

// Buffered ...
func (c *conn) Buffered() int {
	return c.w.Buffered()
}

// Close ...
func (c *conn) Close() error {
	c.w.Close()
	c.r.Close()
	return nil
}

// MTU ...
func (c *conn) MTU() int {
	return connMTU
}

// Read ...
func (c *conn) Read(p []byte) (int, error) {
	return c.r.Read(p)
}

// NewUnbufferedConnPair ...
func NewUnbufferedConnPair() (Conn, Conn) {
	ar, aw := newBufPipe()
	br, bw := newBufPipe()

	return &unbufferedConn{conn: conn{ar, bw}}, &unbufferedConn{conn: conn{br, aw}}
}

var reading, writing int64

type unbufferedConn struct {
	mu sync.Mutex
	conn
}

func (c *unbufferedConn) Read(p []byte) (int, error) {
	atomic.AddInt64(&reading, 1)
	defer atomic.AddInt64(&reading, -1)
	return c.conn.Read(p)
}

func (c *unbufferedConn) Write(p []byte) (int, error) {
	atomic.AddInt64(&writing, 1)
	defer atomic.AddInt64(&writing, -1)
	c.mu.Lock()
	defer c.mu.Unlock()

	n, err := c.conn.Write(p)
	if err != nil {
		return 0, err
	}
	if err := c.conn.Flush(); err != nil {
		return 0, err
	}
	return n, nil
}

func Reading() int64 {
	return atomic.LoadInt64(&reading)
}

func Writing() int64 {
	return atomic.LoadInt64(&writing)
}
