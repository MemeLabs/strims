package ppspp

import (
	"container/heap"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type binQueue struct {
	v binQueueHeap
}

func (q *binQueue) Push(b binmap.Bin, t timeutil.Time) {
	v := binQueueHeapItemPool.Get().(*binQueueHeapItem)
	v.b = b
	v.t = t
	heap.Push(&q.v, v)
}

func (q *binQueue) IterateLessThan(t timeutil.Time) binQueueIterator {
	return binQueueIterator{q, t, binmap.None}
}

type binQueueIterator struct {
	q *binQueue
	t timeutil.Time
	b binmap.Bin
}

func (t *binQueueIterator) Next() (ok bool) {
	if len(t.q.v) == 0 || t.q.v[0].t > t.t {
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
	t timeutil.Time
}

type binQueueHeap []*binQueueHeapItem

func (h binQueueHeap) Len() int { return len(h) }

func (h binQueueHeap) Less(i, j int) bool {
	return h[i].t < h[j].t
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
