// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package store

import (
	"errors"
	"runtime"
	"sync"

	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/mathutil"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/rope"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

// errors ...
var (
	ErrBufferUnderrun     = errors.New("buffer underrun")
	ErrStreamReset        = errors.New("stream reset")
	ErrBinDataNotSet      = errors.New("bin data not set")
	ErrClosed             = errors.New("cannot read from closed buffer")
	ErrReadOffsetNotFound = errors.New("viable read offset not found")
)

type BufferLayout byte

const (
	_ BufferLayout = iota
	CircularBufferLayout
	ElasticBufferLayout
)

// NewBuffer ...
func NewBuffer(size, chunkSize int) (*Buffer, error) {
	return NewBufferWithLayout(size, chunkSize, CircularBufferLayout)
}

func NewBufferWithLayout(size, chunkSize int, layout BufferLayout) (*Buffer, error) {
	if size&(size-1) != 0 {
		return nil, errors.New("buffer size must be power of 2")
	}

	b := &Buffer{
		chunkSize: uint64(chunkSize),
		mask:      uint64(size) - 1,
		size:      binmap.Bin(size * 2),
		head:      binmap.Bin(size * 2),
		bins:      binmap.New(),
		layout:    layout,
		next:      binmap.None,
		ready:     make(chan struct{}),
	}

	switch layout {
	case CircularBufferLayout:
		b.buf = make([]byte, size*chunkSize)
	case ElasticBufferLayout:
		b.buf = make([]byte, mathutil.Max(chunkSize, 1024))
	default:
		return nil, errors.New("unsupported buffer layout")
	}

	return b, nil
}

// Buffer ...
type Buffer struct {
	chunkSize uint64
	mask      uint64
	lock      sync.Mutex
	size      binmap.Bin
	head      binmap.Bin
	bins      *binmap.Map
	buf       []byte
	layout    BufferLayout
	isReady   bool
	ready     chan struct{}
	next      binmap.Bin
	sem       uint64
	err       error
	readers   []chan error
}

// Reset ...
func (s *Buffer) Reset() {
	s.lock.Lock()
	defer s.lock.Unlock()

	if s.bins.Empty() {
		return
	}

	s.head = s.size
	s.bins = binmap.New()
	s.next = 0
	s.sem++

	s.isReady = false
	s.ready = make(chan struct{})

	s.swapReadable(ErrStreamReset)
	s.pushReadable(nil)
}

// Consume ...
func (s *Buffer) Consume(c Chunk) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.set(c.Bin, c.Data)
}

// Close ...
func (s *Buffer) Close() {
	s.swapReadable(ErrClosed)
	s.setReady()
}

func (s *Buffer) setReady() {
	if !s.isReady {
		s.isReady = true
		close(s.ready)
	}
}

func (s *Buffer) swapReadable(err error) {
	s.err = err
	for _, r := range s.readers {
		swapChanValue(r, err)
	}
}

func (s *Buffer) pushReadable(err error) {
	s.err = err
	for _, r := range s.readers {
		select {
		case r <- err:
		default:
		}
	}
}

func (s *Buffer) set(b binmap.Bin, d []byte) {
	l, r := b.Base()
	if l < s.head-s.size {
		return
	}
	if s.layout == ElasticBufferLayout {
		if r > s.size {
			return
		}
		if n := int((r.BaseOffset() + 1) * s.chunkSize); n > len(s.buf) {
			buf := s.buf
			s.buf = make([]byte, mathutil.Max(n, len(s.buf)*4/3))
			copy(s.buf, buf)
		}
	}
	if h := r + 2; s.head < h {
		s.head = h
	}

	copy(s.buf[s.index(b):], d)
	s.bins.Set(b)

	if s.next < s.tail() {
		s.swapReadable(ErrBufferUnderrun)
		return
	}

	if b.Contains(s.next) {
		next := s.bins.FindEmptyAfter(s.next)
		if next.IsNone() {
			next = s.bins.RootBin().BaseRight() + 2
		}
		s.next = next

		s.pushReadable(nil)
	}
}

type DataWriter interface {
	WriteData(m codec.Data) (int, error)
}

// WriteData ...
func (s *Buffer) WriteData(b binmap.Bin, t timeutil.Time, w DataWriter) (int, error) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if s.contains(b) {
		i := s.index(b)
		return w.WriteData(codec.Data{
			Address:   codec.Address(b),
			Timestamp: codec.Timestamp{Time: t},
			Data:      s.buf[i : i+int(b.BaseLength()*s.chunkSize)],
		})
	}
	return 0, ErrBinDataNotSet
}

// SetOffset sets the read offset to the first contiguous filled bin <= b and
// the next expected bin to the next empty bin >= b.
func (s *Buffer) SetOffset(b binmap.Bin) {
	s.lock.Lock()
	defer s.lock.Unlock()

	s.sem++

	b = b.BaseLeft()
	next := s.bins.FindEmptyAfter(b)
	prev := next
	for ; s.bins.FilledAt(b); b = b.LayerLeft() {
		b = s.bins.Cover(b).BaseLeft()
		prev = b
	}

	s.next = next
	s.head = prev + s.size
	s.bins.FillBefore(prev)
	s.setReady()
}

func (s *Buffer) recover() error {
	if s.err != ErrBufferUnderrun {
		return s.err
	}

	next := s.bins.FindFilledAfter(s.tail())
	if next.IsNone() {
		return ErrReadOffsetNotFound
	}

	s.head = next + s.size
	s.bins.FillBefore(next)

	next = s.bins.FindEmptyAfter(next)
	if next.IsNone() {
		next = s.bins.RootBin().BaseRight() + 2
	}
	s.next = next

	s.sem++
	s.swapReadable(nil)

	return nil
}

// Empty ...
func (s *Buffer) Empty() bool {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.bins.Empty()
}

// Bins ...
func (s *Buffer) Bins() *binmap.Map {
	s.lock.Lock()
	defer s.lock.Unlock()

	b := s.bins.Clone()
	b.ResetBefore(s.tail())
	return b
}

func (s *Buffer) index(b binmap.Bin) int {
	return int((uint64(b.BaseOffset()) & s.mask) * s.chunkSize)
}

func (s *Buffer) Tail() binmap.Bin {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.tail()
}

func (s *Buffer) tail() binmap.Bin {
	if s.next.IsNone() {
		return binmap.None
	}
	return s.head - s.size
}

func (s *Buffer) contains(b binmap.Bin) bool {
	return s.tail() <= b.BaseLeft() && b.BaseRight() < s.head && s.bins.FilledAt(b)
}

func (s *Buffer) Next() binmap.Bin {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.next
}

func (s *Buffer) ImportCache(b []byte) error {
	s.lock.Lock()
	defer s.lock.Unlock()

	if s.layout == ElasticBufferLayout {
		s.buf = make([]byte, len(b))
	}

	copy(s.buf, b)

	s.next = byteBin(uint64(len(b)), s.chunkSize)

	s.bins.FillBefore(s.next)
	s.setReady()
	return nil
}

func (s *Buffer) ExportCache() ([]byte, error) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if s.tail() != 0 {
		return nil, errors.New("cannot cache truncated swarm buffer")
	}
	if s.bins.Empty() {
		return nil, errors.New("cannot cache empty buffer")
	}

	b := make([]byte, binByte(s.next, s.chunkSize))
	copy(b, s.buf)
	return b, nil
}

func NewBufferReader(buf *Buffer) *BufferReader {
	r := &BufferReader{
		buf:      buf,
		sem:      buf.sem,
		prev:     buf.tail(),
		off:      binByte(buf.tail(), buf.chunkSize),
		err:      buf.err,
		readable: make(chan error, 1),
	}

	runtime.SetFinalizer(r, bufferReaderFinalizer)

	buf.lock.Lock()
	buf.readers = append(buf.readers, r.readable)
	buf.lock.Unlock()

	return r
}

func bufferReaderFinalizer(b *BufferReader) {
	b.Close()
}

type BufferReader struct {
	buf      *Buffer
	sem      uint64
	prev     binmap.Bin
	off      uint64
	err      error
	readable chan error
	stopper  ioutil.Stopper
}

func (r *BufferReader) sync() {
	if r.sem != r.buf.sem {
		r.sem = r.buf.sem
		r.prev = r.buf.tail()
		r.off = binByte(r.buf.tail(), r.buf.chunkSize)
		r.err = r.buf.err
	}
}

func (r *BufferReader) Unread() {
	r.buf.lock.Lock()
	defer r.buf.lock.Unlock()

	if r.buf.next == binmap.None {
		return
	}

	r.prev = r.buf.tail()
	r.off = binByte(r.buf.tail(), r.buf.chunkSize)

	swapChanValue(r.readable, nil)
}

// Offset ...
func (r *BufferReader) Offset() uint64 {
	<-r.buf.ready

	r.buf.lock.Lock()
	defer r.buf.lock.Unlock()

	r.sync()
	return r.off
}

// Recover ...
func (r *BufferReader) Recover() (uint64, error) {
	if r.err == nil || r.err != ErrBufferUnderrun {
		return 0, r.err
	}

	r.buf.lock.Lock()
	defer r.buf.lock.Unlock()

	if r.sem == r.buf.sem {
		if err := r.buf.recover(); err != nil {
			return 0, err
		}
	}

	off := r.off
	r.sync()
	return r.off - off, nil
}

// SetReadStopper ...
func (r *BufferReader) SetReadStopper(ch ioutil.Stopper) {
	r.stopper = ch
}

func (r *BufferReader) Read(p []byte) (int, error) {
	if r.err != nil {
		return 0, r.err
	}

	r.buf.lock.Lock()
	for r.buf.next == r.prev || len(r.readable) != 0 {
		r.buf.lock.Unlock()

		select {
		case err := <-r.readable:
			if err != nil {
				r.err = err
				return 0, err
			}
		case <-r.stopper:
			return 0, ioutil.ErrStopped
		}

		r.buf.lock.Lock()
	}
	r.sync()
	defer r.buf.lock.Unlock()

	l := int(r.off - binByte(r.buf.tail(), r.buf.chunkSize))
	h := int(binByte(r.buf.next-r.buf.tail(), r.buf.chunkSize))
	i := r.buf.index(r.buf.tail())

	n := rope.New(p).Copy(rope.New(r.buf.buf[i:], r.buf.buf[:i]).Slice(l, h)...)

	r.off += uint64(n)
	r.prev = byteBin(r.off, r.buf.chunkSize)

	return n, nil
}

func (r *BufferReader) Close() error {
	if r.err != nil {
		return r.err
	}

	r.buf.lock.Lock()
	defer r.buf.lock.Unlock()

	for i, c := range r.buf.readers {
		if c == r.readable {
			l := len(r.buf.readers) - 1
			r.buf.readers[i] = r.buf.readers[l]
			r.buf.readers[l] = nil
			r.buf.readers = r.buf.readers[:l]
			break
		}
	}

	r.err = ErrClosed
	swapChanValue(r.readable, ErrClosed)

	return nil
}

func swapChanValue[T any](ch chan T, v T) {
	for {
		select {
		case ch <- v:
			return
		default:
			<-ch
		}
	}
}

func binByte(b binmap.Bin, chunkSize uint64) uint64 {
	return uint64(b/2) * chunkSize
}

func byteBin(n, chunkSize uint64) binmap.Bin {
	return binmap.Bin(n*2) / binmap.Bin(chunkSize)
}
