package store

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/byterope"
)

// Reader ...
type Reader struct {
	chunkSize uint64
	prev      binmap.Bin
	off       uint64
	b         *Buffer
}

// Offset ...
func (r *Reader) Offset() uint64 {
	return r.off
}

// Read ...
func (r *Reader) Read(p []byte) (int, error) {
	r.b.cond.L.Lock()
	defer r.b.cond.L.Unlock()

	if r.b.next == r.prev {
		r.b.cond.Wait()
	}

	l := int(r.off - binByte(r.b.tail(), r.chunkSize))
	h := int(binByte(r.b.next-r.b.tail(), r.chunkSize))
	i := r.b.index(r.b.tail())
	n := byterope.New(p).Copy(byterope.New(r.b.buf[i:], r.b.buf[:i]).Slice(l, h)...)

	r.off += uint64(n)
	r.prev = byteBin(r.off, r.chunkSize)

	return n, nil
}
