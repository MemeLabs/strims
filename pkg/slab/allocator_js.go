//go:build js

// between go 1.17 and 1.18 something changed in the go gc and holding memory
// like we do here prevents the runtime from freeing memory pages causing the
// tab to oom...

package slab

func NewWithSize[T any](size int) *Allocator[T] {
	return &Allocator[T]{}
}

func New[T any]() *Allocator[T] {
	return &Allocator[T]{}
}

type Allocator[T any] struct{}

func (a *Allocator[T]) Alloc() *T { return new(T) }

func (a *Allocator[T]) Free(t *T) {}
