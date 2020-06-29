package store

import (
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/byterope"
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
		cond:      sync.Cond{L: &sync.Mutex{}},
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
	cond      sync.Cond
	next      binmap.Bin
	debug     bool
}

// Consume ...
func (s *Buffer) Consume(c Chunk) {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()

	n := copy(s.buf[s.index(c.Bin):], c.Data)

	if n < len(c.Data) {
		copy(s.buf, c.Data[n:])
	}

	h := c.Bin.BaseRight() + 2
	if s.head < h {
		s.head = h
	}

	s.bins.Set(c.Bin)
	if c.Bin == s.next {
		next := s.bins.FindEmptyAfter(s.next)
		if next.IsNone() {
			next = s.next + 2
		}
		s.next = next

		s.cond.Broadcast()
	}
}

// Slice ...
func (s *Buffer) Slice(b binmap.Bin) (d byterope.Rope, ok bool) {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()

	if s.contains(b) {
		l := int(binByte(b.BaseLeft()-s.tail(), s.chunkSize))
		h := int(binByte(b.BaseRight()-s.tail(), s.chunkSize) + s.chunkSize)
		i := s.index(s.tail())
		return byterope.New(s.buf[i:], s.buf[:i]).Slice(l, h), true
	}
	return
}

// Find ...
func (s *Buffer) Find(b binmap.Bin) (data []byte, ok bool) {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()

	if b.Base() && s.contains(b) {
		i := s.index(b)
		return s.buf[i : i+int(s.chunkSize)], true
	}
	return
}

// ReadBin ...
func (s *Buffer) ReadBin(b binmap.Bin, p []byte) bool {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()

	if b.Base() && s.contains(b) {
		i := s.index(b)
		copy(p, s.buf[i:i+int(s.chunkSize)])
		return true
	}
	return false
}

func (s *Buffer) SetNext(b binmap.Bin) {
	s.next = b
}

// FilledAt ...
func (s *Buffer) FilledAt(b binmap.Bin) bool {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()
	return s.bins.FilledAt(b)
}

// EmptyAt ...
func (s *Buffer) EmptyAt(b binmap.Bin) bool {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()
	return s.bins.EmptyAt(b)
}

// Cover ...
func (s *Buffer) Cover(b binmap.Bin) binmap.Bin {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()
	return s.bins.Cover(b)
}

// Bins ...
func (s *Buffer) Bins() *binmap.Map {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()
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

// Reader ...
func (s *Buffer) Reader() *Reader {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()
	s.cond.Wait()

	return &Reader{
		chunkSize: s.chunkSize,
		prev:      s.next - 2,
		off:       binByte(s.next-2, s.chunkSize),
		b:         s,
	}
}
