package encoding

import (
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/byterope"
)

// newChunkBuffer ...
func newChunkBuffer(n, chunkSize int) (c *chunkBuffer, err error) {
	if n&(n-1) != 0 {
		return nil, errors.New("buffer size must be power of 2")
	}

	c = &chunkBuffer{
		size:      uint64(n),
		chunkSize: uint64(chunkSize),
		mask:      uint64(n) - 1,
		head:      binmap.Bin(n * 2),
		bins:      binmap.New(),
		buf:       make([]byte, n*chunkSize),
		cond:      sync.Cond{L: &sync.Mutex{}},
	}
	return
}

type chunkBuffer struct {
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

func (s *chunkBuffer) Set(b binmap.Bin, p []byte) {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()

	n := copy(s.buf[s.index(b):], p)

	if n < len(p) {
		copy(s.buf, p[n:])
	}

	h := b.BaseRight() + 2
	if s.head < h {
		s.head = h
	}

	s.bins.Set(b)
	if b == s.next {
		next := s.bins.FindEmptyAfter(s.next)
		if next.IsNone() {
			next = s.next + 2
		}
		s.next = next

		s.cond.Broadcast()
	}
}

func (s *chunkBuffer) Slice(b binmap.Bin) (d byterope.Rope, ok bool) {
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

func (s *chunkBuffer) Find(b binmap.Bin) (data []byte, ok bool) {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()

	if b.Base() && s.contains(b) {
		i := s.index(b)
		return s.buf[i : i+int(s.chunkSize)], true
	}
	return
}

func (s *chunkBuffer) index(b binmap.Bin) int {
	return int((uint64(b.BaseOffset()) & s.mask) * s.chunkSize)
}

func (s *chunkBuffer) tail() binmap.Bin {
	return s.head - binmap.Bin(s.size*2)
}

func (s *chunkBuffer) contains(b binmap.Bin) bool {
	return s.tail() <= b.BaseLeft() && b.BaseRight() < s.head && s.bins.FilledAt(b)
}

func (s *chunkBuffer) Reader() *ChunkBufferReader {
	s.cond.L.Lock()
	defer s.cond.L.Unlock()
	s.cond.Wait()

	return &ChunkBufferReader{
		chunkSize: s.chunkSize,
		prev:      s.next - 2,
		off:       binByte(s.next-2, s.chunkSize),
		b:         s,
	}
}

// ChunkBufferReader ...
type ChunkBufferReader struct {
	chunkSize uint64
	prev      binmap.Bin
	off       uint64
	b         *chunkBuffer
}

// Offset ...
func (r *ChunkBufferReader) Offset() uint64 {
	return r.off
}

// Read ...
func (r *ChunkBufferReader) Read(p []byte) (n int, err error) {
	r.b.cond.L.Lock()
	defer r.b.cond.L.Unlock()

	if r.b.next == r.prev {
		r.b.cond.Wait()
	}

	l := int(r.off - binByte(r.b.tail(), r.chunkSize))
	h := int(binByte(r.b.next-r.b.tail(), r.chunkSize))
	i := r.b.index(r.b.tail())
	n = byterope.New(p).Copy(byterope.New(r.b.buf[i:], r.b.buf[:i]).Slice(l, h)...)

	r.off += uint64(n)
	r.prev = byteBin(r.off, r.chunkSize)

	return
}

func binByte(b binmap.Bin, chunkSize uint64) uint64 {
	return uint64(b/2) * chunkSize
}

func byteBin(b, chunkSize uint64) binmap.Bin {
	return binmap.Bin(b*2) / binmap.Bin(chunkSize)
}
