package hashmap

import (
	"bytes"
	"hash/maphash"

	"github.com/MemeLabs/go-ppspp/pkg/slab"
)

const (
	minSize = 16

	minFillFactor = 0.25
	maxFillFactor = 0.75
	resizeRate    = 2
)

type Interface[K any] interface {
	Hash(k K) uint64
	Equal(a, b K) bool
}

func NewByteInterface[T ~[]byte]() Interface[T] {
	m := &byteInterface[T]{}
	m.h.SetSeed(maphash.MakeSeed())
	return m
}

type byteInterface[T ~[]byte] struct {
	h maphash.Hash
}

func (m *byteInterface[T]) Hash(k T) uint64 {
	m.h.Reset()
	m.h.Write([]byte(k))
	return m.h.Sum64()
}

func (m *byteInterface[T]) Equal(a, b T) bool {
	return bytes.Equal([]byte(a), []byte(b))
}

func New[K, V any](iface Interface[K]) Map[K, V] {
	return Map[K, V]{
		lenBounds: computeLendBounds(minSize),
		mask:      uint64(minSize - 1),
		v:         make([]*hashMapItem[K, V], minSize),
		iface:     iface,
		allocator: slab.New[hashMapItem[K, V]](),
	}
}

func computeLendBounds(size int) lenBounds {
	min := float64(size) * minFillFactor
	max := float64(size) * maxFillFactor
	if min < minSize {
		min = 0
	}
	return lenBounds{int(min), int(max)}
}

type lenBounds struct{ min, max int }

type Map[K, V any] struct {
	len       int
	lenBounds lenBounds
	mask      uint64
	v         []*hashMapItem[K, V]
	iface     Interface[K]
	allocator *slab.Allocator[hashMapItem[K, V]]
}

type hashMapItem[K, V any] struct {
	i    uint
	list *hashMapItem[K, V]
	k    K
	v    V
}

func (e *hashMapItem[K, V]) find(k K, iface Interface[K]) *hashMapItem[K, V] {
	if e == nil {
		return nil
	}
	if iface.Equal(e.k, k) {
		return e
	}
	return e.list.find(k, iface)
}

func (e *hashMapItem[K, V]) delete(k K, iface Interface[K]) *hashMapItem[K, V] {
	if e == nil {
		return nil
	}
	if iface.Equal(e.k, k) {
		return e
	}

	ed := e.list.delete(k, iface)
	if ed != nil && ed == e.list {
		e.list = ed.list
	}
	return ed
}

func (l *Map[K, V]) index(k K) uint {
	return uint(l.iface.Hash(k) & l.mask)
}

func (l *Map[K, V]) alloc() (e *hashMapItem[K, V]) {
	l.len++
	if l.len > l.lenBounds.max {
		l.resize(len(l.v) * resizeRate)
	}
	return l.allocator.Alloc()
}

func (l *Map[K, V]) free(e *hashMapItem[K, V]) {
	*e = hashMapItem[K, V]{}
	l.allocator.Free(e)
	l.len--
	if l.len < l.lenBounds.min {
		l.resize(len(l.v) / resizeRate)
	}
}

func (l *Map[K, V]) resize(size int) {
	v := l.v

	l.lenBounds = computeLendBounds(size)
	l.mask = uint64(size - 1)
	l.v = make([]*hashMapItem[K, V], size)

	for _, e := range v {
		for e != nil {
			el := e.list

			i := l.index(e.k)
			e.i = i
			e.list = l.v[i]
			l.v[i] = e

			e = el
		}
	}
}

func (l *Map[K, V]) Len() int {
	return l.len
}

func (l *Map[K, V]) Cap() int {
	return len(l.v)
}

func (l *Map[K, V]) Has(k K) bool {
	return l.v[l.index(k)].find(k, l.iface) != nil
}

func (l *Map[K, V]) Set(k K, v V) {
	i := l.index(k)
	if e := l.v[i].find(k, l.iface); e != nil {
		e.v = v
		return
	}

	e := l.alloc()
	e.k = k
	e.v = v
	e.i = i
	e.list = l.v[i]
	l.v[i] = e
}

func (l *Map[K, V]) Get(k K) (v V, ok bool) {
	if e := l.v[l.index(k)].find(k, l.iface); e != nil {
		return e.v, true
	}
	return
}

func (l *Map[K, V]) Delete(k K) (v V, ok bool) {
	i := l.index(k)
	ed := l.v[i].delete(k, l.iface)
	if ok = ed != nil; ok {
		v = ed.v
		if ed == l.v[i] {
			l.v[i] = ed.list
		}
		l.free(ed)
	}
	return
}

func (l *Map[K, V]) Iterate() Iterator[K, V] {
	return Iterator[K, V]{
		m: l,
	}
}

func (l *Map[K, V]) Keys() []K {
	ks := make([]K, l.len)
	for it := l.Iterate(); it.Next(); {
		ks = append(ks, it.Key())
	}
	return ks
}

func (l *Map[K, V]) Values() []V {
	vs := make([]V, l.len)
	for it := l.Iterate(); it.Next(); {
		vs = append(vs, it.Value())
	}
	return vs
}

type Iterator[K, V any] struct {
	m       *Map[K, V]
	i       int
	v, next *hashMapItem[K, V]
}

func (it *Iterator[K, V]) Next() bool {
	if it.next != nil {
		it.v = it.next
		it.next = it.v.list
		return true
	}

	for i := it.i; i < len(it.m.v); i++ {
		if v := it.m.v[i]; v != nil {
			it.i = i + 1
			it.v = v
			it.next = v.list
			return true
		}
	}

	it.i = len(it.m.v)
	it.v = nil
	return false
}

func (it *Iterator[K, V]) Key() K {
	return it.v.k
}

func (it *Iterator[K, V]) Value() V {
	return it.v.v
}
