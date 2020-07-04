package ppspptest

import (
	"bytes"
	"errors"
	"sync"
)

var errBufPipeClosed = errors.New("io on closed bufPipe")

func newBufPipe() (*bufPipeReader, *bufPipeWriter) {
	b := &bufPipe{close: make(chan struct{})}
	ch := make(chan int, 128)
	return &bufPipeReader{ch: ch, buf: b}, &bufPipeWriter{ch: ch, buf: b}
}

type bufPipe struct {
	close  chan struct{}
	lock   sync.Mutex
	closed bool
	buf    bytes.Buffer
}

func (b *bufPipe) Write(p []byte) (int, error) {
	b.lock.Lock()
	defer b.lock.Unlock()

	if b.closed {
		return 0, errBufPipeClosed
	}

	return b.buf.Write(p)
}

func (b *bufPipe) Read(p []byte) (int, error) {
	b.lock.Lock()
	defer b.lock.Unlock()

	if b.closed {
		return 0, errBufPipeClosed
	}

	return b.buf.Read(p)
}

func (b *bufPipe) Close() error {
	b.lock.Lock()
	defer b.lock.Unlock()

	if b.closed {
		return errBufPipeClosed
	}

	b.closed = true
	close(b.close)
	return nil
}

func (b *bufPipe) NotifyClose() <-chan struct{} {
	return b.close
}

type bufPipeWriter struct {
	ch  chan int
	n   int
	buf *bufPipe
}

func (w *bufPipeWriter) Write(p []byte) (int, error) {
	n, _ := w.buf.Write(p)
	w.n += n
	return n, nil
}

func (w *bufPipeWriter) Flush() error {
	w.ch <- w.n
	w.n = 0
	return nil
}

func (w *bufPipeWriter) Close() error {
	return w.buf.Close()
}

type bufPipeReader struct {
	ch  <-chan int
	n   int
	buf *bufPipe
}

func (r *bufPipeReader) Read(p []byte) (int, error) {
	if r.n == 0 {
		select {
		case r.n = <-r.ch:
		case <-r.buf.NotifyClose():
			return 0, errBufPipeClosed
		}
	}

	if len(p) > r.n {
		p = p[:r.n]
	}

	n, err := r.buf.Read(p)
	r.n -= n
	return n, err
}

func (r *bufPipeReader) Close() error {
	return r.buf.Close()
}
