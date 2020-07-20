package queue

import (
	"sync/atomic"
)

// AtomicRing ...
type AtomicRing struct {
	size  uint64
	mask  uint64
	start uint64
	end   uint64
}

// NewAtomicRing ...
func NewAtomicRing(size uint64) (r *AtomicRing) {
	r = &AtomicRing{
		size: size,
		mask: ^size - 1,
	}

	if r.size&^r.mask != 0 {
		panic("ring size should be power of 2")
	}
	return
}

// Alloc ...
func (r *AtomicRing) Alloc() (i uint64, ok bool) {
	for {
		start := atomic.LoadUint64(&r.start)
		end := atomic.LoadUint64(&r.end)

		if start+r.size > end {
			if ok = atomic.CompareAndSwapUint64(&r.end, end, end+1); ok {
				return end &^ r.mask, true
			}
		}
	}
}

// Push ...
func (r *AtomicRing) Push(i uint64) (ok bool) {
	for {
		if ok = atomic.CompareAndSwapUint64(&r.end, i-1, i); ok {
			return
		}
	}
}

// Pop ...
func (r *AtomicRing) Pop() (i uint64, ok bool) {
	for {
		start := atomic.LoadUint64(&r.start)
		end := atomic.LoadUint64(&r.end)

		if start < end {
			if ok = atomic.CompareAndSwapUint64(&r.start, start, start+1); ok {
				return start &^ r.mask, true
			}
		}
	}
}
