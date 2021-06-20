package vpn

import (
	"hash/maphash"
	"math/bits"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

// TODO: maybe configurable fill ratio/growth rate

func newMessageIDLRU(size int, ttl time.Duration) *messageIDLRU {
	size = 1 << bits.Len(uint(size))
	l := &messageIDLRU{
		ttl:    ttl,
		nextGC: timeutil.Now().Add(ttl),
		mask:   uint64(size - 1),
		v:      make([]*messageIDLRUItem, size),
	}
	l.h.SetSeed(maphash.MakeSeed())
	return l
}

type messageIDLRU struct {
	mu          sync.Mutex
	len         int
	mask        uint64
	ttl         time.Duration
	nextGC      timeutil.Time
	h           maphash.Hash
	v           []*messageIDLRUItem
	freeTop     *messageIDLRUItem
	first, last *messageIDLRUItem
}

type messageIDLRUItem struct {
	id   MessageID
	i    uint
	t    timeutil.Time
	list *messageIDLRUItem
	next *messageIDLRUItem
	prev *messageIDLRUItem
}

func (e *messageIDLRUItem) find(id MessageID) *messageIDLRUItem {
	if e == nil {
		return nil
	}
	if e.id == id {
		return e
	}
	return e.list.find(id)
}

func (e *messageIDLRUItem) remove(v *messageIDLRUItem) *messageIDLRUItem {
	if e == v {
		return e.list
	}
	if e != nil {
		e.list = e.list.remove(v)
	}
	return e
}

func (l *messageIDLRU) index(id MessageID) uint {
	l.h.Reset()
	l.h.Write(id[:])
	return uint(l.h.Sum64() & l.mask)
}

func (l *messageIDLRU) alloc() (e *messageIDLRUItem) {
	l.len++
	if l.len*3/4 > len(l.v) {
		l.grow()
	}

	if l.freeTop == nil {
		return &messageIDLRUItem{}
	}
	e = l.freeTop
	l.freeTop = e.next
	e.next = nil
	return e
}

func (l *messageIDLRU) free(e *messageIDLRUItem) {
	l.len--
	e.list = nil
	e.prev = nil
	e.next = l.freeTop
	l.freeTop = e
}

func (l *messageIDLRU) grow() {
	size := len(l.v) * 2
	l.mask = uint64(size - 1)
	l.v = make([]*messageIDLRUItem, size)

	e := l.first
	for e != nil {
		i := l.index(e.id)
		e.i = i
		e.list = l.v[i]
		l.v[i] = e

		e = e.next
	}
}

func (l *messageIDLRU) prune(t timeutil.Time) {
	e := l.first
	for e != nil && e.t.Before(t) {
		l.v[e.i] = l.v[e.i].remove(e)
		ne := e.next
		l.free(e)
		e = ne
	}
	l.first = e
	if e == nil {
		l.last = nil
	} else {
		e.prev = nil
	}
}

func (l *messageIDLRU) Len() int {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.len
}

func (l *messageIDLRU) Contains(id MessageID) bool {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.v[l.index(id)].find(id) != nil
}

func (l *messageIDLRU) Insert(id MessageID) bool {
	l.mu.Lock()
	defer l.mu.Unlock()

	t := timeutil.Now()
	if t.After(l.nextGC) {
		l.nextGC = t.Add(l.ttl)
		l.prune(t.Add(-l.ttl))
	}

	i := l.index(id)
	if e := l.v[i].find(id); e != nil {
		e.t = t
		if l.last == e {
			return false
		}

		if e.prev != nil {
			e.prev.next = e.next
		}
		if e.next != nil {
			e.next.prev = e.prev
		}
		if l.first == e {
			l.first = e.next
		}
		e.next = nil
		e.prev = l.last
		l.last.next = e
		l.last = e

		return false
	}

	e := l.alloc()
	e.id = id
	e.i = i
	e.t = t
	e.list = l.v[i]
	l.v[i] = e

	if l.last != nil {
		e.prev = l.last
		l.last.next = e
	}
	l.last = e
	if l.first == nil {
		l.first = e
	}

	return true
}
