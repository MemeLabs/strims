package bytereader

import "io"

// New ...
func New(r io.Reader) io.ByteReader {
	return reader{Reader: r}
}

type reader struct {
	io.Reader
	b [1]byte
}

func (r reader) ReadByte() (byte, error) {
	_, err := r.Read(r.b[:])
	return r.b[0], err
}
