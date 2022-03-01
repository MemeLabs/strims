package chunkstream

import (
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/mathutil"
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
	}, nil
}

type ZeroPadWriter struct {
	Writer
}

func (c *ZeroPadWriter) Overhead(n int) int {
	if d := n % len(c.buf); d != 0 {
		return len(c.buf) - d
	}
	return 0
}

func (c *ZeroPadWriter) Flush() (err error) {
	if err = c.Writer.Flush(); err != nil {
		return err
	}

	if c.woff != 0 {
		_, err = ioutil.WriteZerosN(c, int64(len(c.buf)-c.off))
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
		buf:    make([]byte, mathutil.Min(size, 4*1024)),
	}, nil
}

type ZeroPadReader struct {
	Reader
	buf []byte
}

func (c *ZeroPadReader) discard(n int) error {
	for n > 0 {
		buf := c.buf
		if n < len(buf) {
			buf = buf[:n]
		}

		nn, err := c.Reader.Read(buf)
		n -= nn
		if err != nil {
			return err
		}
	}
	return nil
}

func (c *ZeroPadReader) Read(p []byte) (int, error) {
	n, err := c.Reader.Read(p)
	if err == io.EOF && c.off != 0 {
		if err := c.discard(c.size - c.off); err != nil {
			return n, err
		}
	}

	return n, err
}
