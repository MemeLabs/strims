package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/queue"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type peerDataQueueItem struct {
	cs peerWriter
	b  binmap.Bin
	ts timeutil.Time
}

type peerDataQueue struct {
	r   queue.Ring[peerDataQueueItem]
	len int
}

func (q *peerDataQueue) Push(cs peerWriter, b binmap.Bin, ts timeutil.Time) {
	q.r.Push(peerDataQueueItem{cs, b, ts})
	q.len++
}

func (q *peerDataQueue) PushFront(cs peerWriter, b binmap.Bin, ts timeutil.Time) {
	q.r.PushFront(peerDataQueueItem{cs, b, ts})
	q.len++
}

func (q *peerDataQueue) Remove(cs peerWriter, b binmap.Bin) {
	for it := q.r.Iterator(); it.Next(); {
		i := it.Ref()
		if i.cs == cs && b.Contains(i.b) {
			i.cs = nil
			q.len--
		}
	}
}

func (q *peerDataQueue) Pop() (peerWriter, binmap.Bin, timeutil.Time, bool) {
	for {
		i, ok := q.r.PopFront()
		if !ok {
			return nil, binmap.None, timeutil.NilTime, false
		}
		if i.cs != nil {
			q.len--
			return i.cs, i.b, i.ts, true
		}
	}
}

func (q *peerDataQueue) Empty() bool {
	return q.len == 0
}
