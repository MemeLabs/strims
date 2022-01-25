package syncutil

import (
	"sync/atomic"
	"unsafe"
)

func NewPointer[T any](p *T) Pointer[T] {
	return Pointer[T]{p: unsafe.Pointer(p)}
}

type Pointer[T any] struct {
	p unsafe.Pointer
}

func (p *Pointer[T]) Get() *T {
	return (*T)(atomic.LoadPointer(&p.p))
}

func (p *Pointer[T]) Swap(v *T) *T {
	return (*T)(atomic.SwapPointer(&p.p, unsafe.Pointer(v)))
}
