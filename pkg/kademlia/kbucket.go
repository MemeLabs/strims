package kademlia

import (
	"math/bits"
	"sort"
	"sync"
)

// Interface ...
type Interface interface {
	ID() ID
}

// Evictable ...
type Evictable interface {
	Evict()
}

type bucket []Interface

// KBucket ...
type KBucket struct {
	k  int
	id ID
	b  []bucket
	l  int
}

// NewKBucket ...
func NewKBucket(id ID, k int) *KBucket {
	b := make([]bucket, idBitLength)
	is := make([]Interface, k*idBitLength)
	for i := 0; i < idBitLength; i++ {
		b[i] = bucket(is[i*k : i*k])
	}

	v := &KBucket{
		k:  k,
		id: id,
		b:  b,
	}
	return v
}

// Reset ...
func (k *KBucket) Reset() {
	for i, b := range k.b {
		for j := range b {
			b[j] = nil
		}
		k.b[i] = b[:0]
	}
}

// Slice ...
func (k *KBucket) Slice() []Interface {
	n := 0
	for _, b := range k.b {
		n += len(b)
	}

	is := make([]Interface, 0, n)
	for _, b := range k.b {
		is = append(is, b...)
	}

	return is
}

// Insert ...
func (k *KBucket) Insert(n Interface) bool {
	i := k.idBucket(n.ID())

	for j := range k.b[i] {
		if n.ID().Equals(k.b[i][j].ID()) {
			return false
		}
	}

	l := len(k.b[i])
	if l == k.k {
		maxDistance := k.id.XOr(n.ID())
		maxIndex := -1
		for j, n := range k.b[i] {
			distance := k.id.XOr(n.ID())
			if maxDistance.Less(distance) {
				maxDistance = distance
				maxIndex = j
			}
		}

		if maxIndex == -1 {
			return false
		}

		if e, ok := k.b[i][maxIndex].(Evictable); ok {
			e.Evict()
		}
		k.b[i][maxIndex] = n
	} else {
		k.b[i] = k.b[i][:l+1]
		k.b[i][l] = n
		k.l++
	}
	return true
}

// Remove ...
func (k *KBucket) Remove(id ID) bool {
	i := k.idBucket(id)
	for j, n := range k.b[i] {
		if n.ID().Equals(id) {
			copy(k.b[i][j:], k.b[i][j+1:])
			k.b[i][len(k.b[i])-1] = nil
			k.b[i] = k.b[i][:len(k.b[i])-1]
			k.l--
			return true
		}
	}
	return false
}

func (k *KBucket) idBucket(id ID) int {
	for i, v := range k.id.XOr(id) {
		if v != 0 {
			return i*64 + bits.LeadingZeros64(v)
		}
	}
	return idBitLength - 1
}

// Closest ...
func (k *KBucket) Closest(id ID, is []Interface) int {
	f := NewFilter(id)
	defer f.Free()

	i := k.idBucket(id)
	f.Push(k.b[i]...)
	for l, r := i, i; f.Len() < len(is) && (l >= 0 || r < len(k.b)); {
		if l--; l >= 0 {
			f.Push(k.b[l]...)
		}
		if r++; r < len(k.b) {
			f.Push(k.b[r]...)
		}
	}

	return f.Copy(is)
}

// Get ...
func (k *KBucket) Get(id ID) (Interface, bool) {
	i := k.idBucket(id)
	for j, n := range k.b[i] {
		if n.ID().Equals(id) {
			return k.b[i][j], true
		}
	}
	return nil, false
}

// Empty ...
func (k *KBucket) Empty() bool {
	return k.l == 0
}

// NewFilter ...
func NewFilter(id ID) Filter {
	s := filterSlicePool.Get().(*filterSlice)
	*s = (*s)[:0]
	return Filter{
		id:    id,
		items: s,
	}
}

// Filter ...
type Filter struct {
	id    ID
	items *filterSlice
	dirty bool
}

// Push ...
func (s *Filter) Push(ns ...Interface) {
	for _, n := range ns {
		*s.items = append(*s.items, filterSliceItem{n, n.ID().XOr(s.id)})
	}
	s.dirty = true
}

// Len ...
func (s *Filter) Len() int {
	return s.items.Len()
}

// Sort ...
func (s *Filter) Sort() {
	if s.dirty {
		sort.Sort(*s.items)
		s.dirty = false
	}
}

// Pop ...
func (s *Filter) Pop() (Interface, bool) {
	s.Sort()

	n := len(*s.items)
	if n == 0 {
		return nil, false
	}
	out := (*s.items)[n-1]
	(*s.items) = (*s.items)[:n-1]
	return out.Interface, true
}

// Copy ...
func (s *Filter) Copy(out []Interface) int {
	s.Sort()

	l := len(*s.items)
	n := l
	if n > len(out) {
		n = len(out)
	}

	for i := 0; i < n; i++ {
		out[i] = (*s.items)[l-i-1].Interface
	}
	return n
}

// Free ...
func (s *Filter) Free() {
	filterSlicePool.Put(s.items)
}

var filterSlicePool = sync.Pool{
	New: func() interface{} {
		return &filterSlice{}
	},
}

type filterSliceItem struct {
	Interface
	d ID
}

type filterSlice []filterSliceItem

func (h filterSlice) Len() int           { return len(h) }
func (h filterSlice) Less(i, j int) bool { return h[j].d.Less(h[i].d) }
func (h filterSlice) Swap(i, j int)      { h[i], h[j] = h[j], h[i] }
