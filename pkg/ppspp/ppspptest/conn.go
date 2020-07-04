package ppspptest

const connMTU = 1 << 14

// NewConnPair ...
func NewConnPair() (*Conn, *Conn) {
	ar, aw := newBufPipe()
	br, bw := newBufPipe()

	return &Conn{ar, bw}, &Conn{br, aw}
}

// Conn ...
type Conn struct {
	r *bufPipeReader
	w *bufPipeWriter
}

// Write ...
func (c *Conn) Write(p []byte) (int, error) {
	return c.w.Write(p)
}

// Flush ...
func (c *Conn) Flush() error {
	return c.w.Flush()
}

// Close ...
func (c *Conn) Close() error {
	return nil
}

// MTU ...
func (c *Conn) MTU() int {
	return connMTU
}

// Read ...
func (c *Conn) Read(p []byte) (int, error) {
	return c.r.Read(p)
}
