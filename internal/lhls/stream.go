package lhls

import (
	"errors"
	"io"
	"log"
	"sync"

	"github.com/nareix/joy4/av"
	"github.com/nareix/joy4/format"
	"github.com/nareix/joy4/format/ts"
)

func init() {
	format.RegisterAll()
}

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
	opt            StreamOptions
	audioCodecData av.AudioCodecData
	videoCodecData av.VideoCodecData
	lock           sync.RWMutex
	header         []av.CodecData
	segments       []*Segment
	index          uint64
}

// NewStream ...
func NewStream(opt StreamOptions) (s *Stream) {
	s = &Stream{
		opt:      opt,
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
func (l *Stream) Range() (low uint64, high uint64) {
	l.lock.RLock()
	defer l.lock.RUnlock()

	high = l.index
	if high >= uint64(l.opt.HistorySize) {
		low = high - uint64(l.opt.HistorySize)
	}
	return
}

// NextWriter ...
func (l *Stream) NextWriter() io.WriteCloser {
	l.lock.Lock()
	defer l.lock.Unlock()

	l.index++
	i := l.index % uint64(l.opt.HistorySize)
	l.segments[i].Reset()

	return l.segments[i]
}

// WriteHeader ...
func (l *Stream) WriteHeader(header []av.CodecData) {
	l.header = header
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
		segment := l.NextWriter()
		muxer := ts.NewMuxer(segment)
		if err = muxer.WriteHeader(l.header); err != nil {
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
func (l *Stream) SegmentReader(i uint64) (r io.Reader, err error) {
	min, max := l.Range()
	if i < min || i > max {
		return nil, ErrNotFound
	}

	r = &SegmentReader{src: l.segments[i%uint64(l.opt.HistorySize)]}
	return
}
