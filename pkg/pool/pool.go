package pool

import (
	"math"
	"math/bits"
	"sync"
)

// New ...
func New(n int) *Pool {
	p := &Pool{
		n:     n,
		zones: make([]*sync.Pool, n),
	}

	for i := 0; i < n; i++ {
		size := 1 << (16 - i)
		p.zones[i] = &sync.Pool{
			New: func() interface{} {
				b := make([]byte, size)
				return &b
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

// MaxSize ...
func (p *Pool) MaxSize() int {
	return math.MaxUint16
}

var nilBytes []byte

// Get ...
func (p *Pool) Get(size int) (b *[]byte) {
	if size == 0 {
		return &nilBytes
	} else if i := 16 - bits.Len32(uint32(size-1)); i < p.n {
		b = p.zones[i].Get().(*[]byte)
	} else {
		b = p.zones[p.n-1].Get().(*[]byte)
	}

	*b = (*b)[:size]
	return b
}

// Put ...
func (p *Pool) Put(b *[]byte) {
	if i := 16 - bits.TrailingZeros32(uint32(cap(*b))); i < p.n {
		p.zones[i].Put(b)
	} else {
		p.zones[p.n-1].Put(b)
	}
}
