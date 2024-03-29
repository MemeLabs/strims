// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package hls

import (
	"errors"
	"io"
	"sync"
)

// errors ...
var (
	ErrNotFound = errors.New("not found")
)

// Segment ...
type Segment struct {
	cond   sync.Cond
	buf    []byte
	closed bool
}

// NewSegment ...
func NewSegment() *Segment {
	return &Segment{
		cond: sync.Cond{L: &sync.Mutex{}},
	}
}

// Reset ...
func (m *Segment) Reset() {
	m.cond.L.Lock()
	defer m.cond.L.Unlock()

	m.closed = true
	m.cond.Broadcast()

	m.closed = false
	m.buf = m.buf[:0]
}

// Write ...
func (m *Segment) Write(p []byte) (n int, err error) {
	m.cond.L.Lock()
	defer m.cond.L.Unlock()

	m.buf = append(m.buf, p...)
	m.cond.Broadcast()
	return len(p), nil
}

// Close ...
func (m *Segment) Close() (err error) {
	m.cond.L.Lock()
	defer m.cond.L.Unlock()

	m.closed = true
	m.cond.Broadcast()
	return
}

// ReadAt ...
func (m *Segment) ReadAt(p []byte, off int64) (n int, err error) {
	low := int(off)
	high := low + len(p)

	m.cond.L.Lock()
	for {
		if high >= len(m.buf) && !m.closed {
			m.cond.Wait()
		}

		if high >= len(m.buf) {
			if !m.closed {
				continue
			}
			high = len(m.buf)
			err = io.EOF
		}
		break
	}

	buf := m.buf
	m.cond.L.Unlock()

	n = copy(p, buf[low:high])

	return
}

// Len ...
func (m *Segment) Len() int {
	m.cond.L.Lock()
	defer m.cond.L.Unlock()
	return len(m.buf)
}

// SegmentReader ...
type SegmentReader struct {
	src io.ReaderAt
	off int
}

// SegmentReader ...
func (m *SegmentReader) Read(p []byte) (n int, err error) {
	n, err = m.src.ReadAt(p, int64(m.off))
	m.off += n
	return
}

// StreamOptions ...
type StreamOptions struct {
	HistorySize int
}

// DefaultStreamOptions ...
var DefaultStreamOptions = StreamOptions{
	HistorySize: 5,
}

// Stream ...
type Stream struct {
	opt      StreamOptions
	lock     sync.RWMutex
	init     *Segment
	segments []*Segment
	index    uint64
	dm       discontinuityMap
}

// NewStream ...
func NewStream(opt StreamOptions) (s *Stream) {
	s = &Stream{
		opt:      opt,
		init:     NewSegment(),
		segments: make([]*Segment, opt.HistorySize),
	}

	for i := 0; i < opt.HistorySize; i++ {
		s.segments[i] = NewSegment()
	}

	return
}

// NewDefaultStream ...
func NewDefaultStream() *Stream {
	return NewStream(DefaultStreamOptions)
}

// Range ...
func (l *Stream) Range() (low, high uint64, dm discontinuityMap) {
	l.lock.RLock()
	defer l.lock.RUnlock()

	high = l.index
	if high >= uint64(l.opt.HistorySize) {
		low = high - uint64(l.opt.HistorySize)
	}
	return low, high, l.dm
}

// InitWriter ...
func (l *Stream) InitWriter() io.WriteCloser {
	return l.init
}

// InitReader ...
func (l *Stream) InitReader() io.Reader {
	return &SegmentReader{src: l.init}
}

func (l *Stream) MarkDiscontinuity() {
	l.lock.Lock()
	defer l.lock.Unlock()

	l.index++
	l.dm.Advance()
	l.dm.Set()
}

// NextWriter ...
func (l *Stream) NextWriter() *Segment {
	l.lock.Lock()
	defer l.lock.Unlock()

	i := l.index % uint64(l.opt.HistorySize)
	l.index++
	l.dm.Advance()

	l.segments[i].Reset()
	return l.segments[i]
}

// SegmentReader ...
func (l *Stream) SegmentReader(i uint64) (io.Reader, error) {
	min, max, dm := l.Range()
	if i < min || i >= max || dm.Get(i, max) {
		return nil, ErrNotFound
	}

	return &SegmentReader{src: l.segments[i%uint64(l.opt.HistorySize)]}, nil
}

type discontinuityMap uint64

func (m *discontinuityMap) Advance() {
	*m <<= 1
}

func (m *discontinuityMap) Set() {
	*m |= 1
}

func (m *discontinuityMap) Get(i, max uint64) bool {
	return *m&(1<<(max-i)) != 0
}
