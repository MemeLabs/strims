package lhls

import (
	"errors"
	"io"
	"log"
	"sync"

	"github.com/nareix/joy4/av"
	"github.com/nareix/joy4/format/ts"
)

// errors ...
var (
	ErrAudioTrackNotFound = errors.New("audio track missing")
	ErrVideoTrackNotFound = errors.New("video track missing")
	ErrNotFound           = errors.New("not found")
)

// Segment ...
type Segment struct {
	cond   sync.Cond
	buf    []byte
	closed bool
}

// NewSegment ...
func NewSegment(n int) *Segment {
	return &Segment{
		cond: sync.Cond{L: &sync.Mutex{}},
		buf:  make([]byte, 0, n),
	}
}

// Write ...
func (m *Segment) Write(p []byte) (n int, err error) {
	m.cond.L.Lock()
	m.buf = append(m.buf, p...)
	m.cond.Broadcast()
	m.cond.L.Unlock()
	return len(p), nil
}

// Close ...
func (m *Segment) Close() (err error) {
	m.cond.L.Lock()
	m.closed = true
	m.cond.Broadcast()
	m.cond.L.Unlock()
	return
}

// ReadAt ...
func (m *Segment) ReadAt(p []byte, off int64) (n int, err error) {
	m.cond.L.Lock()

	buf := m.buf

	low := int(off)
	if low == len(m.buf) && !m.closed {
		m.cond.Wait()
	}

	high := low + len(p)
	if high >= len(m.buf) {
		high = len(m.buf)

		if m.closed {
			err = io.EOF
		}
	}

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
	BufferSize  int
	HistorySize int
}

// DefaultStreamOptions ...
var DefaultStreamOptions = StreamOptions{
	BufferSize:  4 * 1024 * 1024,
	HistorySize: 10,
}

// Stream ...
type Stream struct {
	opt            StreamOptions
	audioCodecData av.AudioCodecData
	videoCodecData av.VideoCodecData
	lock           sync.RWMutex
	streams        []av.CodecData
	segments       []*Segment
	index          int
}

// NewStream ...
func NewStream(opt StreamOptions) *Stream {
	return &Stream{opt: opt}
}

// NewDefaultStream ...
func NewDefaultStream() *Stream {
	return &Stream{opt: DefaultStreamOptions}
}

// Range ...
func (l *Stream) Range() (int, int) {
	l.lock.RLock()
	defer l.lock.RUnlock()

	low := l.index - len(l.segments)
	if low < 0 {
		low = 0
	}

	return low, l.index
}

func (l *Stream) nextSegment() io.WriteCloser {
	b := NewSegment(l.opt.BufferSize)

	l.lock.Lock()
	defer l.lock.Unlock()

	l.index++
	if l.index > l.opt.HistorySize {
		copy(l.segments, l.segments[1:])
		l.segments = l.segments[:l.opt.HistorySize-1]
	}
	l.segments = append(l.segments, b)

	return b
}

// WriteHeader ...
func (l *Stream) WriteHeader(streams []av.CodecData) {
	l.streams = streams
}

// CopyPackets ...
func (l *Stream) CopyPackets(src av.PacketReader) (err error) {
	var pkt av.Packet
	for !pkt.IsKeyFrame {
		if pkt, err = src.ReadPacket(); err != nil {
			return
		}
	}

	for {
		segment := l.nextSegment()
		muxer := ts.NewMuxer(segment)
		if err = muxer.WriteHeader(l.streams); err != nil {
			return
		}
		log.Println(pkt.Time, pkt.CompositionTime)
		for {
			if err = muxer.WritePacket(pkt); err != nil {
				return
			}
			if pkt, err = src.ReadPacket(); err != nil {
				return
			}
			if pkt.IsKeyFrame {
				break
			}
		}
		if err = muxer.WriteTrailer(); err != nil {
			return
		}
		if err = segment.Close(); err != nil {
			return
		}
	}
}

// SegmentReader ...
func (l *Stream) SegmentReader(i int) (r io.Reader, err error) {
	l.lock.RLock()
	defer l.lock.RUnlock()

	index := len(l.segments) - (l.index - i)
	if index < 0 || index >= len(l.segments) {
		return nil, ErrNotFound
	}

	r = &SegmentReader{src: l.segments[index]}
	return
}
