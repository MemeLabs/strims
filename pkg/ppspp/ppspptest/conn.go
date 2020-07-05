package ppspptest

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
