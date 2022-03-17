package ppspptest

import (
	"bytes"
	"errors"
	"sync"
)

var ErrBufPipeClosed = errors.New("io on closed bufPipe")

func NewBufPipe() (*BufPipeReader, *BufPipeWriter) {
	b := &bufPipe{close: make(chan struct{})}
	ch := make(chan int, 1024)
	return &BufPipeReader{ch: ch, buf: b}, &BufPipeWriter{ch: ch, buf: b}
}

type bufPipe struct {
	close  chan struct{}
	lock   sync.Mutex
	closed bool
	buf    bytes.Buffer
}

func (b *bufPipe) Grow(n int) {
	b.lock.Lock()
	defer b.lock.Unlock()
	b.buf.Grow(n)
}

func (b *bufPipe) Available() int {
	b.lock.Lock()
	defer b.lock.Unlock()

	buf := b.buf.Bytes()
	return cap(buf) - len(buf)
}

func (b *bufPipe) AvailableBuffer() []byte {
	b.lock.Lock()
	defer b.lock.Unlock()

	buf := b.buf.Bytes()
	return buf[len(buf):]
}

func (b *bufPipe) Write(p []byte) (int, error) {
	b.lock.Lock()
	defer b.lock.Unlock()

	if b.closed {
		return 0, ErrBufPipeClosed
	}

	return b.buf.Write(p)
}

func (b *bufPipe) Read(p []byte) (int, error) {
	b.lock.Lock()
	defer b.lock.Unlock()

	if b.closed {
		return 0, ErrBufPipeClosed
	}

	return b.buf.Read(p)
}

func (b *bufPipe) Close() error {
	b.lock.Lock()
	defer b.lock.Unlock()

	if b.closed {
		return ErrBufPipeClosed
	}

	b.closed = true
	close(b.close)
	return nil
}

func (b *bufPipe) CloseNotify() <-chan struct{} {
	return b.close
}

type BufPipeWriter struct {
	ch  chan int
	n   int
	buf *bufPipe
}

func (w *BufPipeWriter) Grow(n int) {
	w.buf.Grow(n - w.n)
}

func (w *BufPipeWriter) Available() int {
	return w.buf.Available()
}

func (w *BufPipeWriter) AvailableBuffer() []byte {
	return w.buf.AvailableBuffer()
}

func (w *BufPipeWriter) Buffered() int {
	return w.n
}

func (w *BufPipeWriter) Write(p []byte) (int, error) {
	n, err := w.buf.Write(p)
	w.n += n
	return n, err
}

func (w *BufPipeWriter) Flush() error {
	w.ch <- w.n
	w.n = 0
	return nil
}

func (w *BufPipeWriter) Close() error {
	return w.buf.Close()
}

type BufPipeReader struct {
	ch  <-chan int
	n   int
	buf *bufPipe
}

func (r *BufPipeReader) Read(p []byte) (int, error) {
	if r.n == 0 {
		select {
		case r.n = <-r.ch:
		case <-r.buf.CloseNotify():
			return 0, ErrBufPipeClosed
		}
	}

	if len(p) > r.n {
		p = p[:r.n]
	}

	n, err := r.buf.Read(p)
	r.n -= n
	return n, err
}

func (r *BufPipeReader) Close() error {
	return r.buf.Close()
}
