package chunkstream

import (
	"io"
)

// max discard buffer size
const zeroPadSize = 4096

// zero padding source
var padding [4096]byte

func NewZeroPadWriter(w io.Writer) (*ZeroPadWriter, error) {
	return NewZeroPadWriterSize(w, DefaultSize)
}

func NewZeroPadWriterSize(w io.Writer, size int) (*ZeroPadWriter, error) {
	c, err := NewWriterSize(w, size)
	if err != nil {
		return nil, err
	}

	return &ZeroPadWriter{*c}, nil
}

type ZeroPadWriter struct {
	Writer
}

func (c *ZeroPadWriter) Flush() error {
	if err := c.Writer.Flush(); err != nil {
		return err
	}

	for c.woff != 0 {
		i := len(padding)
		if l := len(c.buf) - c.off; l < i {
			i = l
		}

		if _, err := c.Write(padding[:i]); err != nil {
			return err
		}
	}
	return nil
}

func NewZeroPadReader(r io.Reader, offset int64) (*ZeroPadReader, error) {
	return NewZeroPadReaderSize(r, offset, DefaultSize)
}

func NewZeroPadReaderSize(r io.Reader, offset int64, size int) (*ZeroPadReader, error) {
	c, err := NewReaderSize(r, offset, size)
	if err != nil {
		return nil, err
	}

	n := zeroPadSize
	if size < n {
		n = size
	}

	cc := &ZeroPadReader{
		Reader: *c,
		pad:    make([]byte, n),
	}
	return cc, nil
}

type ZeroPadReader struct {
	Reader
	pad []byte
}

func (c *ZeroPadReader) Read(p []byte) (int, error) {
	n, err := c.Reader.Read(p)
	for err == io.EOF && c.off != 0 {
		i := len(c.pad)
		if l := c.size - c.off; l < i {
			i = l
		}

		if _, err := c.Reader.Read(c.pad[:i]); err != nil {
			return n, err
		}
	}
	return n, err
}
