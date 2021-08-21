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

func (l *lru) PeekRecentlyTouched(eol timeutil.Time) lruIterator {
	return lruIterator{next: l.head, eol: eol}
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

func (l *lru) Touch(u keyer) {
	if i := l.items.Get(&lruItem{item: u}); i != nil {
		l.remove(i.(*lruItem))
		l.push(i.(*lruItem))
	}
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

type lruIterator struct {
	cur  *lruItem
	next *lruItem
	eol  timeutil.Time
}

func (l *lruIterator) Next() bool {
	l.cur = l.next
	if l.cur == nil || l.eol.After(l.cur.time) {
		return false
	}
	l.next = l.cur.next

	return true
}

func (l *lruIterator) Value() keyer {
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
