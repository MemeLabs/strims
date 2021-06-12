package ppspp

import (
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

var peerDataQueueItemPool = sync.Pool{
	New: func() interface{} {
		return &peerDataQueueItem{}
	},
}

func putPeerDataQueueItemPool(i *peerDataQueueItem) {
	i.next = nil
	i.cs = nil
	peerDataQueueItemPool.Put(i)
}

type peerDataQueueItem struct {
	next *peerDataQueueItem
	cs   PeerWriter
	bin  binmap.Bin
	time time.Time
}

type peerDataQueue struct {
	head, tail *peerDataQueueItem
}

func (q *peerDataQueue) Push(cs PeerWriter, b binmap.Bin, t time.Time) {
	i := peerDataQueueItemPool.Get().(*peerDataQueueItem)
	i.cs = cs
	i.bin = b
	i.time = t

	if q.head == nil {
		q.head = i
	}

	if q.tail != nil {
		q.tail.next = i
	}
	q.tail = i
}

func (q *peerDataQueue) PushFront(cs PeerWriter, b binmap.Bin, t time.Time) {
	i := peerDataQueueItemPool.Get().(*peerDataQueueItem)
	i.next = q.head
	i.cs = cs
	i.bin = b
	i.time = t

	q.head = i

	if q.tail == nil {
		q.tail = i
	}
}

func (q *peerDataQueue) Remove(cs PeerWriter, b binmap.Bin) {
	var prev, next *peerDataQueueItem
	for i := q.head; i != nil; i = next {
		next = i.next
		if i.cs == cs && b.Contains(i.bin) {
			if prev != nil {
				prev.next = i.next
			}
			if q.head == i {
				q.head = i.next
			}
			if q.tail == i {
				q.tail = prev
			}

			putPeerDataQueueItemPool(i)
		} else {
			prev = i
		}
	}
}

func (q *peerDataQueue) Pop() (PeerWriter, binmap.Bin, time.Time, bool) {
	if q.head == nil {
		return nil, binmap.None, time.Time{}, false
	}

	h := q.head

	q.head = h.next

	if q.tail == h {
		q.tail = nil
	}

	defer putPeerDataQueueItemPool(h)
	return h.cs, h.bin, h.time, true
}

func (q *peerDataQueue) Empty() bool {
	return q.head == nil
}
