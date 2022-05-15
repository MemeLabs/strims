// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package queue

import (
	"github.com/MemeLabs/strims/pkg/rope"
)

func NewRing[T any](n int) (r Ring[T]) {
	r.Resize(n)
	return
}

// Ring[T] ...
type Ring[T any] struct {
	mask int
	low  int
	high int
	v    []T
}

// Cap ...
func (r *Ring[T]) Cap() int {
	return len(r.v)
}

// Len ...
func (r *Ring[T]) Len() int {
	return r.high - r.low
}

// Resize ...
func (r *Ring[T]) Resize(n int) {
	if n&(n-1) != 0 {
		panic("ring size should be power of 2")
	}

	if n < r.Cap() {
		return
	} else if n < 1 {
		n = 1
	}

	v := make([]T, n)
	mask := n - 1

	vi := r.low & mask
	i := r.low & r.mask
	rope.New(v[vi:], v[:vi]).Copy(rope.New(r.v[i:], r.v[:i]).Slice(0, r.Len())...)

	r.mask = mask
	r.v = v
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
	if r.Len() == r.Cap() {
		r.Resize(r.Cap() * 2)
	}
	r.low--
	r.v[r.low&r.mask] = v
}

// Push ...
func (r *Ring[T]) Push(v T) {
	if r.Len() == r.Cap() {
		r.Resize(r.Cap() * 2)
	}
	r.v[r.high&r.mask] = v
	r.high++
}

// PopFront ...
func (r *Ring[T]) PopFront() (v T, ok bool) {
	if ok = r.low < r.high; ok {
		v = r.v[r.low&r.mask]
		var t T
		r.v[r.low&r.mask] = t
		r.low++
	}
	return
}

// Pop ...
func (r *Ring[T]) Pop() (v T, ok bool) {
	if ok = r.low < r.high; ok {
		r.high--
		v = r.v[r.high&r.mask]
		var t T
		r.v[r.high&r.mask] = t
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
