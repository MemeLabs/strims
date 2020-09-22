package kademlia

import (
	"container/heap"
	"math"
	"sync"
)

var bucketBits []ID

func init() {
	bucketBits = make([]ID, idBitLength)
	l := len(bucketBits[0])

	for i := 0; i < idBitLength; i++ {
		j := idBitLength - i - 1
		b := j / 64
		for k := b; k < l; k++ {
			bucketBits[j][k] = math.MaxUint64
		}
		bucketBits[j][b] = 1 << (i & 63)
		bucketBits[j][b] |= (bucketBits[j][b] - 1)
	}
}

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
			k.b[i] = k.b[i][:len(k.b[i])-1]
			k.l--
			return true
		}
	}
	return false
}

func (k *KBucket) idBucket(id ID) int {
	v := k.id.XOr(id)
	for i, b := range bucketBits {
		if b.Less(v) {
			return i
		}
	}
	return 0
}

// // Closest ...
// func (k *KBucket) Closest(id ID, is []Interface) (n int) {
// 	h := nodeHeapPool.Get().(*nodeHeap)
// 	defer nodeHeapPool.Put(h)

// 	h.Reset()
// 	for _, b := range k.b {
// 		for _, bn := range b {
// 			h.HeapPush(nodeHeapItem{bn, bn.ID().XOr(id)})
// 		}
// 	}

// 	for n < len(is) {
// 		v, ok := h.HeapPop()
// 		if !ok {
// 			return
// 		}
// 		is[n] = v.Interface
// 		n++
// 	}
// 	return
// }

// Closest ...
func (k *KBucket) Closest(id ID, is []Interface) (n int) {
	f := NewFilter(id)
	defer f.Free()

	for _, b := range k.b {
		for _, bn := range b {
			f.Push(bn)
		}
	}

	for n < len(is) {
		v, ok := f.Pop()
		if !ok {
			return
		}
		is[n] = v
		n++
	}
	return
}

// Get ...
func (k *KBucket) Get(id ID) (Interface, bool) {
	var ifs [1]Interface
	if n := k.Closest(id, ifs[:]); n == 0 {
		return nil, false
	}
	if !id.Equals(ifs[0].ID()) {
		return nil, false
	}
	return ifs[0], true
}

// Empty ...
func (k *KBucket) Empty() bool {
	return k.l == 0
}

// NewFilter ...
func NewFilter(id ID) Filter {
	h := nodeHeapPool.Get().(*nodeHeap)
	h.Reset()
	return Filter{
		id:   id,
		heap: h,
	}
}

// Filter ...
type Filter struct {
	id   ID
	heap *nodeHeap
}

// Push ...
func (s *Filter) Push(n Interface) {
	s.heap.HeapPush(nodeHeapItem{n, n.ID().XOr(s.id)})
}

// Pop ...
func (s *Filter) Pop() (Interface, bool) {
	v, ok := s.heap.HeapPop()
	if !ok {
		return nil, false
	}
	return v.Interface, true
}

// Free ...
func (s *Filter) Free() {
	nodeHeapPool.Put(s.heap)
}

var nodeHeapPool = sync.Pool{
	New: func() interface{} {
		return &nodeHeap{}
	},
}

type nodeHeapItem struct {
	Interface
	d ID
}

type nodeHeap []nodeHeapItem

func (h *nodeHeap) Reset() {
	*h = (*h)[:0]
}

func (h *nodeHeap) HeapPush(e nodeHeapItem) {
	i := len(*h)
	*h = append(*h, e)

	// use fix instead of heap.Push because the interface{} argument to push
	// confuses escape detection and forces the caller to heap allocate the
	// new value
	heap.Fix(h, i)
}

func (h *nodeHeap) HeapPop() (nodeHeapItem, bool) {
	v := h.Pop()
	if v == nil {
		return nodeHeapItem{}, false
	}
	return v.(nodeHeapItem), true
}

func (h nodeHeap) Len() int           { return len(h) }
func (h nodeHeap) Less(i, j int) bool { return h[j].d.Less(h[i].d) }
func (h nodeHeap) Swap(i, j int)      { h[i], h[j] = h[j], h[i] }

func (h *nodeHeap) Push(v interface{}) {
	*h = append(*h, v.(nodeHeapItem))
}

func (h *nodeHeap) Pop() interface{} {
	n := len(*h)
	if n == 0 {
		return nil
	}
	v := (*h)[n-1]
	*h = (*h)[0 : n-1]
	return v
}
