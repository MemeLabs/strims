package ppspptest

import (
	"compress/gzip"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"os"
	"path"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binaryutil"
	"github.com/MemeLabs/go-ppspp/pkg/bytereader"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
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

const CapLogExt = ".cap"

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
	mu   sync.Mutex
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
	if err := c.writeEvent(code); err != nil {
		return err
	}
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

	c.mu.Lock()
	defer c.mu.Unlock()

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

	c.mu.Lock()
	defer c.mu.Unlock()

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

	c.mu.Lock()
	defer c.mu.Unlock()

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

func CapConnLogDir() string {
	return path.Join(os.TempDir(), "capconn")
}

// NewCapLogWriter ...
func NewCapLogWriter(w io.Writer) *CapLogWriter {
	return NewCapLogWriterSize(w, 4096)
}

// NewCapLogWriterSize ...
func NewCapLogWriterSize(w io.Writer, size int) *CapLogWriter {
	gzw := gzip.NewWriter(w)
	return &CapLogWriter{
		gzw:  gzw,
		w:    ioutil.NewSyncWriter(gzw),
		size: size,
	}
}

// CapLogWriter ...
type CapLogWriter struct {
	mu     sync.Mutex
	gzw    *gzip.Writer
	w      io.Writer
	size   int
	cws    []*capLogConnWriter
	closed bool
}

// Writer ...
func (w *CapLogWriter) Writer() io.Writer {
	w.mu.Lock()
	defer w.mu.Unlock()

	if w.closed {
		panic("fuck")
	}

	id := len(w.cws)
	h := make([]byte, binaryutil.UvarintLen(uint64(id)))
	n := binary.PutUvarint(h, uint64(id))

	b := make([]byte, w.size)
	copy(b, h)

	cw := &capLogConnWriter{
		w:   w.w,
		h:   h,
		buf: b,
		off: n,
	}
	w.cws = append(w.cws, cw)
	return cw
}

// Close ...
func (w *CapLogWriter) Close() error {
	var cws []*capLogConnWriter
	w.mu.Lock()
	cws = append(cws, w.cws...)
	w.mu.Unlock()

	for _, cw := range cws {
		if err := cw.Flush(); err != nil {
			return err
		}
	}

	return w.gzw.Close()
}

type capLogConnWriter struct {
	mu  sync.Mutex
	w   io.Writer
	h   []byte
	buf []byte
	off int
}

func (w *capLogConnWriter) Write(p []byte) (n int, err error) {
	w.mu.Lock()
	defer w.mu.Unlock()

	for n < len(p) {
		dn := copy(w.buf[w.off:], p[n:])
		n += dn
		w.off += dn

		if w.off == len(w.buf) {
			if _, err = w.w.Write(w.buf); err != nil {
				return
			}
			w.off = copy(w.buf, w.h)
		}
	}
	return
}

func (w *capLogConnWriter) Flush() error {
	w.mu.Lock()
	defer w.mu.Unlock()

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
	r, err := gzip.NewReader(r)
	if err != nil {
		return err
	}

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
	HandleEOF()
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
			p.Handler.HandleEOF()
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
		if err != nil {
			return err
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
