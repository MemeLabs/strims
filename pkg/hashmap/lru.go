// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package hashmap

import "github.com/MemeLabs/strims/pkg/timeutil"

func NewLRU[K, V any](iface Interface[K]) LRU[K, V] {
	return LRU[K, V]{
		items: New[K, lruItem[K, V]](iface),
	}
}

type LRU[K, V any] struct {
	items      Map[K, lruItem[K, V]]
	head, tail *mapItem[K, lruItem[K, V]]
}

type lruItem[K, V any] struct {
	v          V
	time       timeutil.Time
	prev, next *mapItem[K, lruItem[K, V]]
}

func (l *LRU[K, V]) delete(i *mapItem[K, lruItem[K, V]]) {
	if l.tail == i {
		l.tail = i.v.prev
	}
	if l.head == i {
		l.head = i.v.next
	}
	if i.v.prev != nil {
		i.v.prev.v.next = i.v.next
	}
	if i.v.next != nil {
		i.v.next.v.prev = i.v.prev
	}
}

func (l *LRU[K, V]) push(i *mapItem[K, lruItem[K, V]]) {
	i.v.time = timeutil.Now()
	i.v.next = l.head
	i.v.prev = nil

	if i.v.next != nil {
		i.v.next.v.prev = i
	}

	l.head = i
	if l.tail == nil {
		l.tail = i
	}
}

func (l *LRU[K, V]) Len() int {
	return l.items.Len()
}

func (l *LRU[K, V]) Cap() int {
	return l.items.Cap()
}

func (l *LRU[K, V]) Has(k K) bool {
	return l.items.Has(k)
}

func (l *LRU[K, V]) Pop(eol timeutil.Time) (v V, ok bool) {
	if l.tail == nil || eol.Before(l.tail.v.time) {
		return
	}

	i := l.tail
	v = i.v.v
	l.delete(i)
	l.items.Delete(i.k)
	return v, true
}

func (l *LRU[K, V]) Touch(k K) bool {
	if i, ok := l.items.get(k); ok {
		l.delete(i)
		l.push(i)
		return true
	}
	return false
}

func (l *LRU[K, V]) Get(k K) (v V, ok bool) {
	if i, ok := l.items.get(k); ok {
		l.delete(i)
		l.push(i)
		return i.v.v, true
	}
	return
}

func (l *LRU[K, V]) GetOrInsert(k K, v V) (V, bool) {
	i, ok := l.items.get(k)
	if ok {
		l.delete(i)
	} else {
		i = l.items.set(k, lruItem[K, V]{v: v})
	}
	l.push(i)
	return i.v.v, ok
}

func (l *LRU[K, V]) Delete(k K) (v V, ok bool) {
	if i, ok := l.items.delete(k); ok {
		l.delete(i)
		defer l.items.free(i)
		return i.v.v, true
	}
	return
}

func (l *LRU[K, V]) Iterate() LRUIterator[K, V] {
	return LRUIterator[K, V]{l.items.Iterate()}
}

func (l *LRU[K, V]) IterateTouchedAfter(eol timeutil.Time) LRUForwardIterator[K, V] {
	return LRUForwardIterator[K, V]{next: l.head, eol: eol}
}

func (l *LRU[K, V]) IterateTouchedBefore(eol timeutil.Time) LRUReverseIterator[K, V] {
	return LRUReverseIterator[K, V]{next: l.tail, eol: eol}
}

func (l *LRU[K, V]) Keys() []K {
	return l.items.Keys()
}

func (l *LRU[K, V]) Values() []V {
	vs := make([]V, l.Len())
	for it := l.Iterate(); it.Next(); {
		vs = append(vs, it.Value())
	}
	return vs
}

type LRUIterator[K, V any] struct {
	it Iterator[K, lruItem[K, V]]
}

func (it *LRUIterator[K, V]) Next() bool {
	return it.it.Next()
}

func (it *LRUIterator[K, V]) Key() K {
	return it.it.Key()
}

func (it *LRUIterator[K, V]) Value() V {
	return it.it.Value().v
}

type LRUForwardIterator[K, V any] struct {
	v    *mapItem[K, lruItem[K, V]]
	next *mapItem[K, lruItem[K, V]]
	eol  timeutil.Time
}

func (it *LRUForwardIterator[K, V]) Next() bool {
	it.v = it.next
	if it.v == nil || it.eol.After(it.v.v.time) {
		return false
	}
	it.next = it.v.v.next

	return true
}

func (it *LRUForwardIterator[K, V]) Key() K {
	return it.v.k
}

func (it *LRUForwardIterator[K, V]) Value() V {
	return it.v.v.v
}

type LRUReverseIterator[K, V any] struct {
	v    *mapItem[K, lruItem[K, V]]
	next *mapItem[K, lruItem[K, V]]
	eol  timeutil.Time
}

func (it *LRUReverseIterator[K, V]) Next() bool {
	it.v = it.next
	if it.v == nil || it.eol.Before(it.v.v.time) {
		return false
	}
	it.next = it.v.v.prev

	return true
}

func (it *LRUReverseIterator[K, V]) Key() K {
	return it.v.k
}

func (it *LRUReverseIterator[K, V]) Value() V {
	return it.v.v.v
}
