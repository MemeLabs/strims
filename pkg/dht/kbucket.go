package dht

import (
	"container/heap"
	"math"
	"sync"
)

var bucketBits []ID

func init() {
	bucketBits = make([]ID, IDBits)
	l := len(bucketBits[0])

	for i := 0; i < IDBits; i++ {
		j := IDBits - i - 1
		b := j >> l
		for k := b; k < l; k++ {
			bucketBits[j][k] = math.MaxUint32
		}
		bucketBits[j][b] = 1 << (i & 0x1f)
		bucketBits[j][b] |= (bucketBits[j][b] - 1)
	}
}

// Interface ...
type Interface interface {
	ID() ID
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
func NewKBucket(n Interface, k int) *KBucket {
	b := make([]bucket, IDBits)
	is := make([]Interface, k*IDBits)
	for i := 0; i < IDBits; i++ {
		b[i] = bucket(is[i*k : i*k])
	}

	v := &KBucket{
		k:  k,
		id: n.ID(),
		b:  b,
	}
	v.Insert(n)
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

	l := len(k.b[i])
	if l == k.k {
		return false
	}

	for j := range k.b[i] {
		if n.ID().Equals(k.b[i][j].ID()) {
			return false
		}
	}

	k.b[i] = k.b[i][:l+1]
	k.b[i][l] = n
	k.l++
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

	panic("bucket for id missing")
}

// Closest ...
func (k *KBucket) Closest(id ID, is []Interface) (n int) {
	h := nodeHeapPool.Get().(*nodeHeap)
	defer nodeHeapPool.Put(h)

	h.Reset()
	for _, b := range k.b {
		for _, bn := range b {
			h.HeapPush(nodeHeapEntry{bn, bn.ID().XOr(id)})
		}
	}

	for n < len(is) {
		v, ok := h.HeapPop()
		if !ok {
			return
		}
		is[n] = v.Interface
		n++
	}
	return
}

var nodeHeapPool = sync.Pool{
	New: func() interface{} {
		return &nodeHeap{}
	},
}

type nodeHeapEntry struct {
	Interface
	d ID
}
type nodeHeap []nodeHeapEntry

func (h *nodeHeap) Reset() {
	*h = (*h)[:0]
}

func (h *nodeHeap) HeapPush(e nodeHeapEntry) {
	i := len(*h)
	*h = append(*h, e)

	// use fix instead of heap.Push because the interface{} argument to push
	// confuses escape detection and forces the caller to heap allocate the
	// new value
	heap.Fix(h, i)
}

func (h *nodeHeap) HeapPop() (nodeHeapEntry, bool) {
	v := h.Pop()
	if v == nil {
		return nodeHeapEntry{}, false
	}
	return v.(nodeHeapEntry), true
}

func (h nodeHeap) Len() int           { return len(h) }
func (h nodeHeap) Less(i, j int) bool { return h[i].d.Less(h[j].d) }
func (h nodeHeap) Swap(i, j int)      { h[i], h[j] = h[j], h[i] }

func (h *nodeHeap) Push(v interface{}) {
	*h = append(*h, v.(nodeHeapEntry))
}

func (h *nodeHeap) Pop() interface{} {
	n := len(*h)
	v := (*h)[n-1]
	*h = (*h)[0 : n-1]
	return v
}
