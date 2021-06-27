package store

import (
	"errors"
	"log"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/byterope"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

// errors ...
var (
	ErrBufferUnderrun     = errors.New("buffer underrun")
	ErrBinDataNotSet      = errors.New("bin data not set")
	ErrClosed             = errors.New("cannot read from closed buffer")
	ErrReadOffsetNotFound = errors.New("viable read offset not found")
)

// NewBuffer ...
func NewBuffer(size, chunkSize int) (c *Buffer, err error) {
	if size&(size-1) != 0 {
		return nil, errors.New("buffer size must be power of 2")
	}

	return &Buffer{
		size:      uint64(size),
		chunkSize: uint64(chunkSize),
		mask:      uint64(size) - 1,
		head:      binmap.Bin(size * 2),
		bins:      binmap.New(),
		buf:       make([]byte, size*chunkSize),
		next:      binmap.None,
		prev:      binmap.None,
		ready:     make(chan struct{}),
		readable:  make(chan error, 1),
	}, nil
}

// Buffer ...
type Buffer struct {
	size      uint64
	chunkSize uint64
	mask      uint64
	head      binmap.Bin
	bins      *binmap.Map
	buf       []byte
	lock      sync.Mutex
	readyOnce sync.Once
	ready     chan struct{}
	readable  chan error
	next      binmap.Bin
	prev      binmap.Bin
	off       uint64
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
	s.readyOnce.Do(func() { close(s.ready) })
}

func (s *Buffer) swapReadable(err error) error {
	var prev error
	for {
		select {
		case s.readable <- err:
			return prev
		default:
			prev = <-s.readable
		}
	}
}

// Set ...
func (s *Buffer) Set(b binmap.Bin, d []byte) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.set(b, d)
}

func (s *Buffer) set(b binmap.Bin, d []byte) {
	copy(s.buf[s.index(b):], d)

	h := b.BaseRight() + 2
	if s.head < h {
		s.head = h
	}

	s.bins.Set(b)
	if !b.Contains(s.next) {
		if s.next < s.tail() {
			select {
			case s.readable <- ErrBufferUnderrun:
			default:
			}
		}
		return
	}

	next := s.bins.FindEmptyAfter(s.next)
	if next.IsNone() {
		next = s.bins.RootBin().BaseRight() + 2
	}
	s.next = next

	select {
	case s.readable <- nil:
	default:
	}
}

// ReadBin ...
func (s *Buffer) ReadBin(b binmap.Bin, p []byte) bool {
	s.lock.Lock()
	defer s.lock.Unlock()

	if s.contains(b) {
		i := s.index(b)
		copy(p, s.buf[i:i+int(b.BaseLength()*s.chunkSize)])
		return true
	}
	return false
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

	b = b.BaseLeft()
	next := s.bins.FindEmptyAfter(b)
	prev := next
	for ; s.bins.FilledAt(b); b = b.LayerLeft() {
		b = s.bins.Cover(b).BaseLeft()
		prev = b
	}

	s.next = next
	s.prev = prev
	s.off = binByte(prev, s.chunkSize)
	s.bins.FillBefore(prev)
	s.setReady()
}

// Recover ...
func (s *Buffer) Recover() (uint64, error) {
	s.lock.Lock()
	defer s.lock.Unlock()

	emptyBins := []binmap.Bin{}
	for it := s.bins.IterateEmpty(); it.Next(); {
		emptyBins = append(emptyBins, it.Value())
	}
	log.Println(emptyBins)

	next := s.bins.FindFilledAfter(s.tail())
	if next.IsNone() {
		return 0, ErrReadOffsetNotFound
	}

	off := s.off
	s.off = binByte(next, s.chunkSize)

	s.prev = next
	s.bins.FillBefore(next)

	next = s.bins.FindEmptyAfter(next)
	if next.IsNone() {
		next = s.bins.RootBin().BaseRight() + 2
	}
	s.next = next

	err := s.swapReadable(nil)
	if err != nil && err != ErrBufferUnderrun {
		return 0, err
	}

	return s.off - off, nil
}

// FilledAt ...
func (s *Buffer) FilledAt(b binmap.Bin) bool {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.bins.FilledAt(b)
}

// EmptyAt ...
func (s *Buffer) EmptyAt(b binmap.Bin) bool {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.bins.EmptyAt(b)
}

// Cover ...
func (s *Buffer) Cover(b binmap.Bin) binmap.Bin {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.bins.Cover(b)
}

// Bins ...
func (s *Buffer) Bins() *binmap.Map {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.bins.Clone()
}

func (s *Buffer) index(b binmap.Bin) int {
	return int((uint64(b.BaseOffset()) & s.mask) * s.chunkSize)
}

func (s *Buffer) tail() binmap.Bin {
	return s.head - binmap.Bin(s.size*2)
}

func (s *Buffer) contains(b binmap.Bin) bool {
	return s.tail() <= b.BaseLeft() && b.BaseRight() < s.head && s.bins.FilledAt(b)
}

func (s *Buffer) Next() binmap.Bin {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.next
}

// Offset ...
func (s *Buffer) Offset() uint64 {
	<-s.ready
	return s.off
}

// Read ...
func (s *Buffer) Read(p []byte) (int, error) {
	s.lock.Lock()
	if s.next == s.prev {
		// HAX: sync.Cond is broken https://github.com/golang/go/issues/21165
		s.lock.Unlock()

		if err := <-s.readable; err != nil {
			return 0, err
		}

		s.lock.Lock()
	}
	defer s.lock.Unlock()

	// if s.next < s.tail() {
	// 	return 0, ErrBufferUnderrun
	// }

	l := int(s.off - binByte(s.tail(), s.chunkSize))
	h := int(binByte(s.next-s.tail(), s.chunkSize))
	i := s.index(s.tail())

	// defer func() {
	// 	if err := recover(); err != nil {
	// 		log.Printf("l %d h %d i %d s.off %d s.next %d s.tail() %d", l, h, i, s.off, s.next, s.tail())
	// 		log.Println(err)
	// 		panic("fuck")
	// 	}
	// }()

	n := byterope.New(p).Copy(byterope.New(s.buf[i:], s.buf[:i]).Slice(l, h)...)

	s.off += uint64(n)
	s.prev = byteBin(s.off, s.chunkSize)

	return n, nil
}

func binByte(b binmap.Bin, chunkSize uint64) uint64 {
	return uint64(b/2) * chunkSize
}

func byteBin(b, chunkSize uint64) binmap.Bin {
	return binmap.Bin(b*2) / binmap.Bin(chunkSize)
}
