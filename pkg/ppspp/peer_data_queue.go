package ppspp

import (
	"sync"

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
}

type peerDataQueue struct {
	head, tail *peerDataQueueItem
}

func (q *peerDataQueue) Push(cs PeerWriter, b binmap.Bin) {
	i := peerDataQueueItemPool.Get().(*peerDataQueueItem)
	i.cs = cs
	i.bin = b

	if q.head == nil {
		q.head = i
	}

	if q.tail != nil {
		q.tail.next = i
	}
	q.tail = i
}

func (q *peerDataQueue) PushFront(cs PeerWriter, b binmap.Bin) {
	i := peerDataQueueItemPool.Get().(*peerDataQueueItem)
	i.next = q.head
	i.cs = cs
	i.bin = b

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

func (q *peerDataQueue) Pop() (PeerWriter, binmap.Bin, bool) {
	if q.head == nil {
		return nil, binmap.None, false
	}

	h := q.head

	q.head = h.next

	if q.tail == h {
		q.tail = nil
	}

	defer putPeerDataQueueItemPool(h)
	return h.cs, h.bin, true
}

func (q *peerDataQueue) Empty() bool {
	return q.head == nil
}
