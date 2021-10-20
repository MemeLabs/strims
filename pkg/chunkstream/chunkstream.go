// Package chunkstream implements io Reader/Writer for serializing streams of
// input segments (like mpeg-ts) by splitting them into fixed length chunks
// with headers indicating whether the next chunk contains the end of an input
// segment and at what offset.
package chunkstream

import (
	"encoding/binary"
	"errors"
	"io"
	"math"
)

// errors ...
var (
	ErrHeaderWritten = errors.New("header already written")
	ErrSizeRange     = errors.New("header interval out of range")
)

const (
	// header byte length
	headerLen = 4

	// flag indicating an end of record in the next chunk
	eorFlag = 0x80
)

// chunk size consts
const (
	MaxSize     = math.MaxUint32>>1 - headerLen
	DefaultSize = math.MaxUint16 - headerLen
)

// NewWriter ...
func NewWriter(w io.Writer) (*Writer, error) {
	return NewWriterSize(w, DefaultSize)
}

// NewWriterSize ...
func NewWriterSize(w io.Writer, size int) (c *Writer, err error) {
	if size > MaxSize {
		return nil, ErrSizeRange
	}

	c = &Writer{
		w:   w,
		buf: make([]byte, size),
		off: headerLen,
	}
	return
}

// Writer ...
type Writer struct {
	w    io.Writer
	buf  []byte
	off  int
	woff int
}

// Write implements io.Writer
func (c *Writer) Write(p []byte) (n int, err error) {
	for {
		dn := copy(c.buf[c.off:], p[n:])
		n += dn
		c.off += dn
		if n == len(p) && c.off < len(c.buf) {
			return
		}

		_, err = c.w.Write(c.buf[c.woff:])
		if err != nil {
			return
		}
		c.off = headerLen
		c.woff = 0
	}
}

// Flush flushes any remaining bytes in the buffer to w. Should be called
// after every input segment and before closing the underlying writer.
func (c *Writer) Flush() (err error) {
	if c.woff != 0 {
		return ErrHeaderWritten
	}

	binary.BigEndian.PutUint32(c.buf, uint32(c.off))
	c.buf[0] |= eorFlag

	c.woff, err = c.w.Write(c.buf[:c.off])
	if err != nil {
		return
	}

	c.buf[0] = 0
	c.buf[1] = 0
	c.buf[2] = 0
	c.buf[3] = 0
	return
}

// NewReader ...
func NewReader(w io.Reader, offset int64) (*Reader, error) {
	return NewReaderSize(w, offset, DefaultSize)
}

// NewReaderSize ...
func NewReaderSize(r io.Reader, offset int64, size int) (c *Reader, err error) {
	if size > MaxSize {
		return nil, ErrSizeRange
	}

	c = &Reader{
		r:      r,
		header: make([]byte, 0, headerLen),
		size:   size,
	}
	c.SetOffset(offset)
	return
}

// Reader ...
type Reader struct {
	r      io.Reader
	header []byte
	size   int // chunk byte length
	off    int // read offset in current chunk
	roff   int // record end index in current chunk (int max when undefined)
}

func (c *Reader) SetOffset(offset int64) {
	c.header = c.header[:0]
	c.off = int(offset % int64(c.size))
	c.roff = math.MaxInt32
}

// Read implements io.Reader
func (c *Reader) Read(p []byte) (n int, err error) {
	if c.off < headerLen {
		n = headerLen - len(c.header)
	} else if len(p) > c.roff-c.off {
		n = c.roff - c.off
	} else if len(p) > c.size-c.off {
		n = c.size - c.off
	}

	if n == 0 || n > len(p) {
		n = len(p)
	}

	rn, err := c.r.Read(p[:n])
	if err != nil {
		return
	}
	n = rn

	if c.off < headerLen {
		n -= c.readHeader(p[:rn])
	}

	c.off += rn
	if c.off == c.roff {
		c.roff = math.MaxInt32
		err = io.EOF
	}
	if c.off == c.size {
		c.off = 0
	}
	return
}

func (c *Reader) readHeader(p []byte) (n int) {
	off := len(c.header)
	c.header = c.header[:headerLen]
	n = copy(c.header[off:], p)

	if off+n < headerLen {
		c.header = c.header[:off+n]
		return
	}

	if c.header[0]&eorFlag != 0 {
		c.header[0] &^= eorFlag
		c.roff = int(binary.BigEndian.Uint32(c.header))
	}

	c.header = c.header[:0]
	return
}
