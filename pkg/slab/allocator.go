package slab

import (
	"math"
	"reflect"
	"unsafe"
)

const (
	DefaultSize = 128
	MaxSize     = math.MaxUint16 >> 1
)

type ref uint16

const nilRef ref = math.MaxUint16

func newStack(size int) *stack {
	refs := make([]ref, size)
	for i := 0; i < size; i++ {
		refs[i] = ref(i)
	}
	return &stack{
		refs: refs,
		i:    ref(size),
	}
}

type stack struct {
	refs []ref
	i    ref
}

func (b *stack) Alloc() ref {
	if b.i == 0 {
		return nilRef
	}
	b.i--
	return b.refs[b.i]
}

func (b *stack) Free(n ref) {
	b.refs[b.i] = n
	b.i++
}

func newSlab[T any](size int) slab[T] {
	data := make([]T, size)

	return slab[T]{
		offset: (*reflect.SliceHeader)(unsafe.Pointer(&data)).Data,
		next:   nilRef,
		data:   data,
		free:   newStack(size),
	}
}

type slab[T any] struct {
	offset uintptr
	next   ref
	data   []T
	free   *stack
}

func NewWithSize[T any](size int) *Allocator[T] {
	if size > MaxSize {
		size = MaxSize
	}
	return &Allocator[T]{
		slabs: []slab[T]{newSlab[T](size)},
		size:  size,
		tsize: unsafe.Sizeof(*new(T)),
	}
}

func New[T any]() *Allocator[T] {
	return NewWithSize[T](DefaultSize)
}

type Allocator[T any] struct {
	slabs []slab[T]
	head  ref
	size  int
	tsize uintptr
}

func (a *Allocator[T]) grow() {
	slab := newSlab[T](a.size)
	a.slabs = append(a.slabs, slab)
	a.head = ref(len(a.slabs) - 1)
	if size := a.size * 2; size < int(nilRef) {
		a.size = size
	}

	for i := 0; i < len(a.slabs); i++ {
		if a.slabs[i].offset > slab.offset {
			copy(a.slabs[i+1:], a.slabs[i:])
			a.slabs[i] = slab
			a.head = ref(i)
			break
		}
	}
}

func (a *Allocator[T]) Alloc() *T {
	ii := a.slabs[a.head].free.Alloc()
	for ii == nilRef {
		if a.slabs[a.head].next == nilRef {
			a.grow()
		} else {
			a.head = a.slabs[a.head].next
			a.slabs[a.head].next = nilRef
		}
		ii = a.slabs[a.head].free.Alloc()
	}

	return &a.slabs[a.head].data[ii]
}

func (a *Allocator[T]) Free(t *T) {
	p := uintptr(unsafe.Pointer(t))

	l := 0
	r := len(a.slabs)
	for l != r {
		m := (r + l) >> 1
		if a.slabs[m].offset <= p {
			l = m + 1
		} else {
			r = m
		}
	}

	i := ref(l - 1)
	ii := (p - a.slabs[i].offset) / a.tsize
	a.slabs[i].free.Free(ref(ii))

	if a.head != i && a.slabs[i].next == nilRef {
		a.slabs[i].next = a.head
		a.head = i
	}
}
