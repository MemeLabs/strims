// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package hashmap

import (
	"bytes"
	"encoding/binary"
	"hash/maphash"
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

func NewUint64Interface[T ~uint64]() Interface[T] {
	m := &uint64Interface[T]{}
	m.h.SetSeed(maphash.MakeSeed())
	return m
}

type uint64Interface[T ~uint64] struct {
	h maphash.Hash
	b [8]byte
}

func (m *uint64Interface[T]) Hash(k T) uint64 {
	binary.BigEndian.PutUint64(m.b[:], uint64(k))
	m.h.Reset()
	m.h.Write(m.b[:])
	return m.h.Sum64()
}

func (m *uint64Interface[T]) Equal(a, b T) bool {
	return a == b
}

func New[K, V any](iface Interface[K]) Map[K, V] {
	return Map[K, V]{
		lenBounds: computeLenBounds(minSize),
		mask:      uint64(minSize - 1),
		v:         make([]*mapItem[K, V], minSize),
		iface:     iface,
	}
}

func computeLenBounds(size int) lenBounds {
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
	v         []*mapItem[K, V]
	iface     Interface[K]
}

type mapItem[K, V any] struct {
	list *mapItem[K, V]
	k    K
	v    V
}

func (e *mapItem[K, V]) find(k K, iface Interface[K]) *mapItem[K, V] {
	if e == nil {
		return nil
	}
	if iface.Equal(e.k, k) {
		return e
	}
	return e.list.find(k, iface)
}

func (e *mapItem[K, V]) delete(k K, iface Interface[K]) *mapItem[K, V] {
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

func (l *Map[K, V]) alloc() (e *mapItem[K, V]) {
	l.len++
	if l.len > l.lenBounds.max {
		l.resize(len(l.v) * resizeRate)
	}
	return &mapItem[K, V]{}
}

func (l *Map[K, V]) free(e *mapItem[K, V]) {
	*e = mapItem[K, V]{}
	l.len--
	if l.len < l.lenBounds.min {
		l.resize(len(l.v) / resizeRate)
	}
}

func (l *Map[K, V]) resize(size int) {
	v := l.v

	l.lenBounds = computeLenBounds(size)
	l.mask = uint64(size - 1)
	l.v = make([]*mapItem[K, V], size)

	for _, e := range v {
		for e != nil {
			el := e.list

			i := l.index(e.k)
			e.list = l.v[i]
			l.v[i] = e

			e = el
		}
	}
}

func (l *Map[K, V]) swap(k K, v V) (e *mapItem[K, V], p V) {
	i := l.index(k)
	if e = l.v[i].find(k, l.iface); e != nil {
		p = e.v
		e.v = v
		return
	}

	e = l.alloc()
	e.k = k
	e.v = v
	e.list = l.v[i]
	l.v[i] = e
	return
}

func (l *Map[K, V]) set(k K, v V) *mapItem[K, V] {
	e, _ := l.swap(k, v)
	return e
}

func (l *Map[K, V]) get(k K) (v *mapItem[K, V], ok bool) {
	if e := l.v[l.index(k)].find(k, l.iface); e != nil {
		return e, true
	}
	return
}

// delete removes the item at k if it exists and returns it. the caller must
// free the item or it will leak.
func (l *Map[K, V]) delete(k K) (ed *mapItem[K, V], ok bool) {
	i := l.index(k)
	ed = l.v[i].delete(k, l.iface)
	if ok = ed != nil; ok {
		if ed == l.v[i] {
			l.v[i] = ed.list
		}
	}
	return
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
	l.set(k, v)
}

func (l *Map[K, V]) Swap(k K, v V) V {
	_, p := l.swap(k, v)
	return p
}

func (l *Map[K, V]) Get(k K) (v V, ok bool) {
	if e := l.v[l.index(k)].find(k, l.iface); e != nil {
		return e.v, true
	}
	return
}

func (l *Map[K, V]) Delete(k K) (v V, ok bool) {
	if ed, ok := l.delete(k); ok {
		defer l.free(ed)
		return ed.v, true
	}
	return
}

func (l *Map[K, V]) Iterate() Iterator[K, V] {
	return Iterator[K, V]{
		m: l,
	}
}

func (l *Map[K, V]) Keys() []K {
	ks := make([]K, 0, l.len)
	for it := l.Iterate(); it.Next(); {
		ks = append(ks, it.Key())
	}
	return ks
}

func (l *Map[K, V]) Values() []V {
	vs := make([]V, 0, l.len)
	for it := l.Iterate(); it.Next(); {
		vs = append(vs, it.Value())
	}
	return vs
}

type Iterator[K, V any] struct {
	m *Map[K, V]
	i int
	v *mapItem[K, V]
}

func (it *Iterator[K, V]) Next() bool {
	if it.v != nil && it.v.list != nil {
		it.v = it.v.list
		return true
	}

	for i := it.i; i < len(it.m.v); i++ {
		if v := it.m.v[i]; v != nil {
			it.i = i + 1
			it.v = v
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
