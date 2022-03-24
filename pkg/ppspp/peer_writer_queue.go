package ppspp

import (
	"sync"
)

var peerTaskRunnerQueueItemPool = sync.Pool{
	New: func() any {
		return &peerTaskRunnerQueueItem{}
	},
}

func putPeerWriterQueueItemPool(i *peerTaskRunnerQueueItem) {
	i.next = nil
	i.c = nil
	peerTaskRunnerQueueItemPool.Put(i)
}

type peerTaskRunnerQueueItem struct {
	next *peerTaskRunnerQueueItem
	c    peerTaskRunner
}

type peerTaskRunnerQueueTicket struct {
	v uint64
}

func (t *peerTaskRunnerQueueTicket) PunchTicket(v uint64) bool {
	if t.v == v {
		return false
	}
	t.v = v
	return true
}

func newPeerTaskRunnerQueue() peerTaskRunnerQueue {
	return peerTaskRunnerQueue{
		v: 1,
	}
}

type peerTaskRunnerQueue struct {
	head, tail *peerTaskRunnerQueueItem
	v          uint64
}

func (q *peerTaskRunnerQueue) Push(c peerTaskRunner) bool {
	if !c.PunchTicket(q.v) {
		return false
	}

	i := peerTaskRunnerQueueItemPool.Get().(*peerTaskRunnerQueueItem)
	i.next = nil
	i.c = c

	q.push(i)
	return true
}

func (q *peerTaskRunnerQueue) push(i *peerTaskRunnerQueueItem) {
	if q.head == nil {
		q.head = i
	}

	if q.tail != nil {
		q.tail.next = i
	}
	q.tail = i
}

func (q *peerTaskRunnerQueue) Remove(c peerTaskRunner) {
	var prev *peerTaskRunnerQueueItem
	for i := q.head; i != nil; i = i.next {
		if i.c == c {
			if prev != nil {
				prev.next = i.next
			}
			if q.head == i {
				q.head = i.next
			}
			if q.tail == i {
				q.tail = prev
			}

			putPeerWriterQueueItemPool(i)
			return
		}

		prev = i
	}
}

func (q *peerTaskRunnerQueue) Detach() detachedPeerWriterQueue {
	c := detachedPeerWriterQueue{
		head: q.head,
	}

	q.head = nil
	q.tail = nil
	q.v++

	return c
}

func (q *peerTaskRunnerQueue) Reattach(dq detachedPeerWriterQueue) {
	for dq.head != nil {
		i := dq.head
		dq.head = i.next

		if i.c.PunchTicket(q.v) {
			q.push(i)
		} else {
			putPeerWriterQueueItemPool(i)
		}
	}
}

func (q *peerTaskRunnerQueue) Empty() bool {
	return q.head == nil
}

type detachedPeerWriterQueue struct {
	head *peerTaskRunnerQueueItem
}

func (q *detachedPeerWriterQueue) Pop() (peerTaskRunner, bool) {
	if q.head == nil {
		return nil, false
	}

	h := q.head

	q.head = h.next

	defer putPeerWriterQueueItemPool(h)
	return h.c, true
}
