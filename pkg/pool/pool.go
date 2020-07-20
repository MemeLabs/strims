package pool

import (
	"errors"
	"math/bits"
	"sync"
)

var errBufferTooSmall = errors.New("buffer too small")

// New ...
func New(n int) *Pool {
	p := &Pool{
		n:     n,
		zones: make([]*sync.Pool, n),
	}

	for i := 0; i < n; i++ {
		size := 1<<(16-i) - 1
		p.zones[i] = &sync.Pool{
			New: func() interface{} {
				return make([]byte, size)
			},
		}
	}

	return p
}

// Pool ...
type Pool struct {
	n     int
	zones []*sync.Pool
}

// Get ...
func (p *Pool) Get(size uint16) (b []byte) {
	if i := bits.LeadingZeros16(size); i < p.n {
		b = p.zones[i].Get().([]byte)
	} else {
		b = p.zones[p.n-1].Get().([]byte)
	}
	return b[:size]
}

// Put ...
func (p *Pool) Put(b []byte) {
	if i := bits.LeadingZeros16(uint16(cap(b))); i < p.n {
		p.zones[i].Put(&b)
	} else {
		p.zones[p.n-1].Put(&b)
	}
}
