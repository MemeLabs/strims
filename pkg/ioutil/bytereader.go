package ioutil

import "io"

// NewByteReader ...
func NewByteReader(r io.Reader) *ByteReader {
	return &ByteReader{Reader: r}
}

type ByteReader struct {
	io.Reader
	b [1]byte
}

func (r ByteReader) ReadByte() (byte, error) {
	_, err := r.Read(r.b[:])
	return r.b[0], err
}
