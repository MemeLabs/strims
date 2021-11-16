package ppspptest

import (
	"sync"
)

const connMTU = 1 << 14

// Conn ...
type Conn interface {
	Write(p []byte) (int, error)
	Flush() error
	Buffered() int
	Close() error
	MTU() int
	SetQOSWeight(w uint64)
	Read(p []byte) (int, error)
}

// NewConnPair ...
func NewConnPair() (Conn, Conn) {
	ar, aw := NewBufPipe()
	br, bw := NewBufPipe()

	return &conn{ar, bw}, &conn{br, aw}
}

// Conn ...
type conn struct {
	r *BufPipeReader
	w *BufPipeWriter
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

func (c *conn) SetQOSWeight(w uint64) {}

// Read ...
func (c *conn) Read(p []byte) (int, error) {
	return c.r.Read(p)
}

// NewUnbufferedConnPair ...
func NewUnbufferedConnPair() (Conn, Conn) {
	ar, aw := NewBufPipe()
	br, bw := NewBufPipe()

	return &unbufferedConn{conn: conn{ar, bw}}, &unbufferedConn{conn: conn{br, aw}}
}

type unbufferedConn struct {
	mu sync.Mutex
	conn
}

func (c *unbufferedConn) Read(p []byte) (int, error) {
	n, err := c.conn.Read(p)
	// fmt.Printf("read %p %s", c, spew.Sdump(p[:n]))
	return n, err
}

func (c *unbufferedConn) Write(p []byte) (int, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	// fmt.Printf("write %p %s", c, spew.Sdump(p))
	n, err := c.conn.Write(p)
	if err != nil {
		return 0, err
	}
	if err := c.conn.Flush(); err != nil {
		return 0, err
	}
	return n, nil
}
