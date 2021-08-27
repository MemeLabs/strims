package chunkstream

import (
	"io"
)

func NewZeroPadWriter(w io.Writer) (*ZeroPadWriter, error) {
	return NewZeroPadWriterSize(w, DefaultSize)
}

func NewZeroPadWriterSize(w io.Writer, size int) (*ZeroPadWriter, error) {
	c, err := NewWriterSize(w, size)
	if err != nil {
		return nil, err
	}

	return &ZeroPadWriter{
		Writer: *c,
		pad:    make([]byte, size),
	}, nil
}

type ZeroPadWriter struct {
	Writer
	pad []byte
}

func (c *ZeroPadWriter) Overhead(n int) int {
	if d := n % len(c.pad); d != 0 {
		return len(c.pad) - d
	}
	return 0
}

func (c *ZeroPadWriter) Flush() (err error) {
	if err = c.Writer.Flush(); err != nil {
		return err
	}

	if c.woff != 0 {
		_, err = c.Write(c.pad[:len(c.pad)-c.off])
	}
	return err
}

func NewZeroPadReader(r io.Reader, offset int64) (*ZeroPadReader, error) {
	return NewZeroPadReaderSize(r, offset, DefaultSize)
}

func NewZeroPadReaderSize(r io.Reader, offset int64, size int) (*ZeroPadReader, error) {
	c, err := NewReaderSize(r, offset, size)
	if err != nil {
		return nil, err
	}

	return &ZeroPadReader{
		Reader: *c,
		pad:    make([]byte, size),
	}, nil
}

type ZeroPadReader struct {
	Reader
	pad []byte
}

func (c *ZeroPadReader) Read(p []byte) (int, error) {
	n, err := c.Reader.Read(p)
	if err == io.EOF && c.off != 0 {
		if _, err := c.Reader.Read(c.pad[:len(c.pad)-c.off]); err != nil {
			return n, err
		}
	}
	return n, err
}
