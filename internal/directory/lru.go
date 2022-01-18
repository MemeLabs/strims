package directory

import (
	"bytes"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/petar/GoLLRB/llrb"
)

type lru struct {
	items llrb.LLRB
	tail  *lruItem
	head  *lruItem
}

func (l *lru) Get(u keyer) keyer {
	if ii := l.items.Get(&lruItem{item: u}); ii != nil {
		i := ii.(*lruItem)
		l.remove(i)
		l.push(i)
		return i.item
	}
	return nil
}

func (l *lru) Each(it func(i keyer) bool) {
	l.items.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		return it(ii.(*lruItem).item)
	})
}

func (l *lru) IterateTouchedAfter(eol timeutil.Time) lruForwardIterator {
	return lruForwardIterator{next: l.head, eol: eol}
}

func (l *lru) IterateTouchedBefore(eol timeutil.Time) lruReverseIterator {
	return lruReverseIterator{next: l.tail, eol: eol}
}

func (l *lru) GetOrInsert(u keyer) keyer {
	i := &lruItem{item: u}
	if ii := l.items.Get(i); ii != nil {
		i = ii.(*lruItem)
		u = i.item
		l.remove(i)
	} else {
		l.items.ReplaceOrInsert(i)
	}
	l.push(i)
	return u
}

func (l *lru) Delete(u keyer) {
	ii := l.items.Delete(&lruItem{item: u})
	if ii == nil {
		return
	}

	i := ii.(*lruItem)
	l.remove(i)
	l.head = i.next
}

func (l *lru) Pop(eol timeutil.Time) keyer {
	if l.tail == nil || eol.Before(l.tail.time) {
		return nil
	}

	i := l.tail
	l.remove(i)
	return i.item
}

func (l *lru) Touch(u keyer) bool {
	if i := l.items.Get(&lruItem{item: u}); i != nil {
		l.remove(i.(*lruItem))
		l.push(i.(*lruItem))
		return true
	}
	return false
}

func (l *lru) remove(i *lruItem) {
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

func (l *lru) push(i *lruItem) {
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

type lruItem struct {
	item keyer
	time timeutil.Time
	prev *lruItem
	next *lruItem
}

func (i *lruItem) Key() []byte {
	return i.item.Key()
}

func (i *lruItem) Less(o llrb.Item) bool {
	return keyerLess(i.item, o)
}

type lruForwardIterator struct {
	cur  *lruItem
	next *lruItem
	eol  timeutil.Time
}

func (l *lruForwardIterator) Next() bool {
	l.cur = l.next
	if l.cur == nil || l.eol.After(l.cur.time) {
		return false
	}
	l.next = l.cur.next

	return true
}

func (l *lruForwardIterator) Value() keyer {
	return l.cur.item
}

type lruReverseIterator struct {
	cur  *lruItem
	next *lruItem
	eol  timeutil.Time
}

func (l *lruReverseIterator) Next() bool {
	l.cur = l.next
	if l.cur == nil || l.eol.Before(l.cur.time) {
		return false
	}
	l.next = l.cur.prev

	return true
}

func (l *lruReverseIterator) Value() keyer {
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

func newIndexedLRU() indexedLRU {
	return indexedLRU{
		index: map[uint64]*lruItem{},
	}
}

type indexedLRU struct {
	lru
	index map[uint64]*lruItem
}

func (l *indexedLRU) GetByID(id uint64) keyer {
	if i, ok := l.index[id]; ok {
		l.remove(i)
		l.push(i)
		return i.item
	}
	return nil
}

func (l *indexedLRU) GetOrInsert(u indexedLRUKeyer) keyer {
	i := &lruItem{item: u}
	if ii := l.items.Get(i); ii != nil {
		i = ii.(*lruItem)
		u = i.item.(indexedLRUKeyer)
		l.remove(i)
	} else {
		l.index[u.ID()] = i
		l.items.ReplaceOrInsert(i)
	}
	l.push(i)
	return u
}

func (l *indexedLRU) Delete(u indexedLRUKeyer) {
	ii := l.items.Delete(&lruItem{item: u})
	if ii == nil {
		return
	}
	delete(l.index, u.ID())

	i := ii.(*lruItem)
	l.remove(i)
	l.head = i.next
}

type indexedLRUKeyer interface {
	keyer
	ID() uint64
}
