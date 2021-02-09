package ppspptest

import (
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/bytereader"
)

// event codes
const (
	CapConnEOF uint8 = iota
	CapConnInit
	CapConnWrite
	CapConnWriteErr
	CapConnFlush
	CapConnFlushErr
	CapConnRead
	CapConnReadErr
)

// NewCapConn ...
func NewCapConn(c Conn, w io.Writer, label string) (*CapConn, error) {
	cc := &CapConn{
		Conn: c,
		t:    time.Unix(0, 0),
		w:    w,
	}
	if err := cc.writeEventWithData(CapConnInit, []byte(label)); err != nil {
		return nil, err
	}
	return cc, nil
}

// CapConn ...
type CapConn struct {
	Conn
	w    io.Writer
	t    time.Time
	temp [1 + binary.MaxVarintLen64]byte
}

func (c *CapConn) writeEvent(code uint8) error {
	t := time.Now()
	d := t.Sub(c.t)
	c.t = t
	c.temp[0] = code
	n := binary.PutUvarint(c.temp[1:], uint64(d))
	_, err := c.w.Write(c.temp[:n+1])
	return err
}

func (c *CapConn) writeEventWithData(code uint8, p []byte) error {
	c.writeEvent(code)
	n := binary.PutUvarint(c.temp[:], uint64(len(p)))
	if _, err := c.w.Write(c.temp[:n]); err != nil {
		return err
	}
	_, err := c.w.Write(p)
	return err
}

// Write ...
func (c *CapConn) Write(p []byte) (int, error) {
	n, err := c.Conn.Write(p)
	if err != nil {
		if cerr := c.writeEventWithData(CapConnWriteErr, []byte(err.Error())); cerr != nil {
			return n, fmt.Errorf("%s (original error %w)", cerr, err)
		}
	} else {
		if cerr := c.writeEventWithData(CapConnWrite, p[:n]); cerr != nil {
			return n, cerr
		}
	}
	return n, err
}

// Flush ...
func (c *CapConn) Flush() error {
	err := c.Conn.Flush()
	if err != nil {
		if cerr := c.writeEventWithData(CapConnFlushErr, []byte(err.Error())); cerr != nil {
			return fmt.Errorf("%s (original error %w)", cerr, err)
		}
	} else {
		if cerr := c.writeEvent(CapConnFlush); cerr != nil {
			return cerr
		}
	}
	return err
}

// Read ...
func (c *CapConn) Read(p []byte) (int, error) {
	n, err := c.Conn.Read(p)
	if err != nil {
		if cerr := c.writeEventWithData(CapConnReadErr, []byte(err.Error())); cerr != nil {
			return n, fmt.Errorf("%s (original error %w)", cerr, err)
		}
	} else {
		if cerr := c.writeEventWithData(CapConnRead, p[:n]); cerr != nil {
			return n, cerr
		}
	}
	return n, err
}

// NewCapLogWriter ...
func NewCapLogWriter(w io.Writer) *CapLogWriter {
	return NewCapLogWriterSize(w, 4096)
}

// NewCapLogWriterSize ...
func NewCapLogWriterSize(w io.Writer, size int) *CapLogWriter {
	return &CapLogWriter{w: w, size: size}
}

// CapLogWriter ...
type CapLogWriter struct {
	w    io.Writer
	size int
	cws  []*capLogConnWriter
}

// Writer ...
func (w *CapLogWriter) Writer() io.Writer {
	h := make([]byte, 2*binary.MaxVarintLen64)
	n := binary.PutUvarint(h, uint64(len(w.cws)))

	b := make([]byte, w.size)
	copy(b, h[:n])

	cw := &capLogConnWriter{w.w, h[:n], b, n}
	w.cws = append(w.cws, cw)
	return cw
}

// Flush ...
func (w *CapLogWriter) Flush() error {
	for _, cw := range w.cws {
		if err := cw.Flush(); err != nil {
			return err
		}
	}
	return nil
}

type capLogConnWriter struct {
	w   io.Writer
	h   []byte
	buf []byte
	off int
}

func (w *capLogConnWriter) Write(p []byte) (n int, err error) {
	for n < len(p) {
		dn := copy(w.buf[w.off:], p[n:])
		n += dn
		w.off += dn

		if n < len(p) {
			if _, err = w.w.Write(w.buf); err != nil {
				return
			}
			w.off = copy(w.buf, w.h)
		}
	}
	return
}

func (w *capLogConnWriter) Flush() error {
	if w.off == len(w.h) {
		return nil
	}

	for i := w.off; i < len(w.buf); i++ {
		w.buf[i] = 0
	}
	_, err := w.w.Write(w.buf)
	return err
}

// ReadCapLog ...
func ReadCapLog(r io.Reader, f func() CapLogHandler) error {
	return ReadCapLogSize(r, 4096, f)
}

// ReadCapLogSize ...
func ReadCapLogSize(r io.Reader, size int, f func() CapLogHandler) error {
	ws := map[uint64]io.WriteCloser{}
	b := make([]byte, size)

	defer func() {
		for _, w := range ws {
			w.Close()
		}
	}()

	for {
		if _, err := io.ReadFull(r, b); err != nil {
			return err
		}

		i, n := binary.Uvarint(b)
		w, ok := ws[i]
		if !ok {
			var r io.ReadCloser
			r, w = io.Pipe()
			ws[i] = w

			go func() {
				(&capLogParser{f()}).Parse(r)
				r.Close()
			}()
		}

		w.Write(b[n:])
	}
}

// CapLogHandler ...
type CapLogHandler interface {
	HandleInit(t time.Time, label string)
	HandleWrite(t time.Time, p []byte)
	HandleWriteErr(t time.Time, err error)
	HandleFlush(t time.Time)
	HandleFlushErr(t time.Time, err error)
	HandleRead(t time.Time, p []byte)
	HandleReadErr(t time.Time, err error)
}

type capLogParser struct {
	Handler CapLogHandler
}

func (p *capLogParser) Parse(r io.Reader) error {
	var code [1]byte
	t := time.Unix(0, 0)

	for {
		if _, err := r.Read(code[:]); err != nil {
			return err
		}

		d, err := binary.ReadUvarint(bytereader.New(r))
		if err != nil {
			return err
		}
		t = t.Add(time.Duration(d))

		switch code[0] {
		case CapConnInit:
			b, err := p.readData(r)
			if err != nil {
				return err
			}
			p.Handler.HandleInit(t, string(b))
		case CapConnEOF:
			return nil
		case CapConnWrite:
			err = p.handleDataEvent(p.Handler.HandleWrite, t, r)
		case CapConnRead:
			err = p.handleDataEvent(p.Handler.HandleRead, t, r)
		case CapConnFlush:
			p.Handler.HandleFlush(t)
		case CapConnWriteErr:
			err = p.handleErrorEvent(p.Handler.HandleWriteErr, t, r)
		case CapConnFlushErr:
			err = p.handleErrorEvent(p.Handler.HandleFlushErr, t, r)
		case CapConnReadErr:
			err = p.handleErrorEvent(p.Handler.HandleReadErr, t, r)
		}
	}
}

func (p *capLogParser) readData(r io.Reader) ([]byte, error) {
	n, err := binary.ReadUvarint(bytereader.New(r))
	if err != nil {
		return nil, err
	}

	b := make([]byte, int(n))
	if _, err := io.ReadFull(r, b); err != nil {
		return nil, err
	}

	return b, nil
}

func (p *capLogParser) handleDataEvent(cb func(time.Time, []byte), t time.Time, r io.Reader) error {
	b, err := p.readData(r)
	if err != nil {
		return err
	}

	cb(t, b)
	return nil
}

func (p *capLogParser) handleErrorEvent(cb func(time.Time, error), t time.Time, r io.Reader) error {
	b, err := p.readData(r)
	if err != nil {
		return err
	}

	cb(t, errors.New(string(b)))
	return nil
}
