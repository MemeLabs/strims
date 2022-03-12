package directory

import (
	"bytes"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/petar/GoLLRB/llrb"
)

type keyerPointer[V any] interface {
	keyer
	*V
}

type lru[V any, T keyerPointer[V]] struct {
	items llrb.LLRB
	tail  *lruItem[V, T]
	head  *lruItem[V, T]
}

func (l *lru[V, T]) Get(u T) T {
	if ii := l.items.Get(&lruItem[V, T]{item: u}); ii != nil {
		i := ii.(*lruItem[V, T])
		l.remove(i)
		l.push(i)
		return i.item
	}
	return nil
}

func (l *lru[V, T]) Has(u T) bool {
	return l.items.Has(&lruItem[V, T]{item: u})
}

func (l *lru[V, T]) Each(it func(i T) bool) {
	l.items.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		return it(ii.(*lruItem[V, T]).item)
	})
}

func (l *lru[V, T]) IterateTouchedAfter(eol timeutil.Time) lruForwardIterator[V, T] {
	return lruForwardIterator[V, T]{next: l.head, eol: eol}
}

func (l *lru[V, T]) IterateTouchedBefore(eol timeutil.Time) lruReverseIterator[V, T] {
	return lruReverseIterator[V, T]{next: l.tail, eol: eol}
}

func (l *lru[V, T]) GetOrInsert(u T) T {
	i := &lruItem[V, T]{item: u}
	if ii := l.items.Get(i); ii != nil {
		i = ii.(*lruItem[V, T])
		l.remove(i)
	} else {
		l.items.ReplaceOrInsert(i)
	}
	l.push(i)
	return i.item
}

func (l *lru[V, T]) Delete(u T) {
	ii := l.items.Delete(&lruItem[V, T]{item: u})
	if ii == nil {
		return
	}

	l.remove(ii.(*lruItem[V, T]))
}

func (l *lru[V, T]) Pop(eol timeutil.Time) T {
	if l.tail == nil || eol.Before(l.tail.time) {
		return nil
	}

	i := l.tail
	l.remove(i)
	return i.item
}

func (l *lru[V, T]) Touch(u T) bool {
	if i := l.items.Get(&lruItem[V, T]{item: u}); i != nil {
		l.remove(i.(*lruItem[V, T]))
		l.push(i.(*lruItem[V, T]))
		return true
	}
	return false
}

func (l *lru[V, T]) remove(i *lruItem[V, T]) {
	if l.tail == i {
		l.tail = i.prev
	}
	if l.head == i {
		l.head = i.next
	}
	if i.prev != nil {
		i.prev.next = i.next
	}
	if i.next != nil {
		i.next.prev = i.prev
	}
}

func (l *lru[V, T]) push(i *lruItem[V, T]) {
	i.time = timeutil.Now()
	i.next = l.head
	i.prev = nil

	if i.next != nil {
		i.next.prev = i
	}

	l.head = i
	if l.tail == nil {
		l.tail = i
	}
}

type lruItem[V any, T keyerPointer[V]] struct {
	item T
	time timeutil.Time
	prev *lruItem[V, T]
	next *lruItem[V, T]
}

func (i *lruItem[V, T]) Key() []byte {
	return i.item.Key()
}

func (i *lruItem[V, T]) Less(o llrb.Item) bool {
	return keyerLess(i.item, o)
}

type lruForwardIterator[V any, T keyerPointer[V]] struct {
	cur  *lruItem[V, T]
	next *lruItem[V, T]
	eol  timeutil.Time
}

func (l *lruForwardIterator[V, T]) Next() bool {
	l.cur = l.next
	if l.cur == nil || l.eol.After(l.cur.time) {
		return false
	}
	l.next = l.cur.next

	return true
}

func (l *lruForwardIterator[V, T]) Value() T {
	return l.cur.item
}

type lruReverseIterator[V any, T keyerPointer[V]] struct {
	cur  *lruItem[V, T]
	next *lruItem[V, T]
	eol  timeutil.Time
}

func (l *lruReverseIterator[V, T]) Next() bool {
	l.cur = l.next
	if l.cur == nil || l.eol.Before(l.cur.time) {
		return false
	}
	l.next = l.cur.prev

	return true
}

func (l *lruReverseIterator[V, T]) Value() T {
	return l.cur.item
}

type lruKey struct {
	key []byte
}

func (l *lruKey) Key() []byte {
	return l.key
}

func (l *lruKey) Less(o llrb.Item) bool {
	return keyerLess(l, o)
}

type keyer interface {
	llrb.Item
	Key() []byte
}

func keyerLess(h keyer, o llrb.Item) bool {
	if o, ok := o.(keyer); ok {
		return bytes.Compare(h.Key(), o.Key()) == -1
	}
	return !o.Less(h)
}

type indexedLRUKeyer interface {
	keyer
	ID() uint64
}

type indexedLRUKeyerPointer[T any] interface {
	indexedLRUKeyer
	*T
}

func newIndexedLRU[V any, T indexedLRUKeyerPointer[V]]() indexedLRU[V, T] {
	return indexedLRU[V, T]{
		index: map[uint64]*lruItem[V, T]{},
	}
}

type indexedLRU[V any, T indexedLRUKeyerPointer[V]] struct {
	lru[V, T]
	index map[uint64]*lruItem[V, T]
}

func (l *indexedLRU[V, T]) GetByID(id uint64) T {
	if i, ok := l.index[id]; ok {
		l.remove(i)
		l.push(i)
		return i.item
	}
	return nil
}

func (l *indexedLRU[V, T]) GetOrInsert(u T) T {
	i := &lruItem[V, T]{item: u}
	if ii := l.items.Get(i); ii != nil {
		i = ii.(*lruItem[V, T])
		l.remove(i)
	} else {
		l.index[i.item.ID()] = i
		l.items.ReplaceOrInsert(i)
	}
	l.push(i)
	return i.item
}

func (l *indexedLRU[V, T]) Delete(u T) {
	ii := l.items.Delete(&lruItem[V, T]{item: u})
	if ii == nil {
		return
	}
	delete(l.index, u.ID())

	i := ii.(*lruItem[V, T])
	l.remove(i)
}
