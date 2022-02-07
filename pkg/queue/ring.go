package queue

import (
	"github.com/MemeLabs/go-ppspp/pkg/rope"
)

func NewRing[T any](n int) (r Ring[T]) {
	r.Resize(n)
	return
}

// Ring[T] ...
type Ring[T any] struct {
	size int
	mask int
	low  int
	high int
	zero T
	v    []T
}

// Size ...
func (r *Ring[T]) Size() int {
	return r.size
}

// Resize ...
func (r *Ring[T]) Resize(size int) {
	if size < r.size {
		return
	} else if size < 1 {
		size = 1
	}

	v := make([]T, size)
	mask := size - 1

	vi := r.low & mask
	i := r.low & r.mask
	rope.New(v[vi:], v[:vi]).Copy(rope.New(r.v[i:], r.v[:i]).Slice(0, r.high-r.low)...)

	r.size = size
	r.mask = mask
	r.v = v

	if r.size&r.mask != 0 {
		panic("ring size should be power of 2")
	}
}

// Len ...
func (r *Ring[T]) Len() int {
	return r.high - r.low
}

// Head ...
func (r *Ring[T]) Head() (v T, ok bool) {
	if r.low < r.high {
		return r.v[(r.high-1)&r.mask], true
	}
	return
}

// Tail ...
func (r *Ring[T]) Tail() (v T, ok bool) {
	if r.low < r.high {
		return r.v[r.low&r.mask], true
	}
	return
}

// PushFront ...
func (r *Ring[T]) PushFront(v T) {
	if r.high-r.low == r.size {
		r.Resize(r.size * 2)
	}
	r.low--
	r.v[r.low&r.mask] = v
}

// Push ...
func (r *Ring[T]) Push(v T) {
	if r.high-r.low == r.size {
		r.Resize(r.size * 2)
	}
	r.v[r.high&r.mask] = v
	r.high++
}

// PopFront ...
func (r *Ring[T]) PopFront() (v T, ok bool) {
	if ok = r.low < r.high; ok {
		v = r.v[r.low&r.mask]
		r.v[r.low&r.mask] = r.zero
		r.low++
	}
	return
}

// Pop ...
func (r *Ring[T]) Pop() (v T, ok bool) {
	if ok = r.low < r.high; ok {
		r.high--
		v = r.v[r.high&r.mask]
		r.v[r.high&r.mask] = r.zero
	}
	return
}

// Iterator ...
func (r *Ring[T]) Iterator() *RingIterator[T] {
	return &RingIterator[T]{
		i: r.low - 1,
		r: r,
	}
}

// RingIterator[T] ...
type RingIterator[T any] struct {
	i int
	r *Ring[T]
}

// Next ...
func (it *RingIterator[T]) Next() bool {
	it.i++
	return it.i < it.r.high
}

// Value ...
func (it *RingIterator[T]) Value() T {
	return it.r.v[it.i&it.r.mask]
}

// Ref ...
func (it *RingIterator[T]) Ref() *T {
	return &it.r.v[it.i&it.r.mask]
}
