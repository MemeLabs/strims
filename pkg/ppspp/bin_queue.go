package ppspp

import (
	"container/heap"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

type binQueue struct {
	v binQueueHeap
}

func (q *binQueue) Push(b binmap.Bin, i int64) {
	v := binQueueHeapItemPool.Get().(*binQueueHeapItem)
	v.b = b
	v.i = i
	heap.Push(&q.v, v)
}

func (q *binQueue) IterateLessThan(i int64) binQueueIterator {
	return binQueueIterator{q, i, binmap.None}
}

type binQueueIterator struct {
	q *binQueue
	i int64
	b binmap.Bin
}

func (t *binQueueIterator) Next() (ok bool) {
	if len(t.q.v) == 0 || t.q.v[0].i > t.i {
		t.b = binmap.None
		return false
	}

	t.b = t.q.v[0].b
	binQueueHeapItemPool.Put(heap.Pop(&t.q.v))
	return true
}

func (t *binQueueIterator) Value() binmap.Bin {
	return t.b
}

var binQueueHeapItemPool = sync.Pool{
	New: func() interface{} {
		return &binQueueHeapItem{}
	},
}

type binQueueHeapItem struct {
	b binmap.Bin
	i int64
}

type binQueueHeap []*binQueueHeapItem

func (h binQueueHeap) Len() int { return len(h) }

func (h binQueueHeap) Less(i, j int) bool {
	return h[i].i < h[j].i
}

func (h binQueueHeap) Swap(i, j int) {
	h[i], h[j] = h[j], h[i]
}

func (h *binQueueHeap) Push(x interface{}) {
	item := x.(*binQueueHeapItem)
	*h = append(*h, item)
}

func (h *binQueueHeap) Pop() interface{} {
	old := *h
	n := len(old)
	item := old[n-1]
	old[n-1] = nil
	*h = old[0 : n-1]
	return item
}
