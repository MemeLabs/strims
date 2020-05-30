package vpn

import (
	"container/list"
	"sync"
	"time"
)

type discardQueueItem interface {
	Deadline() time.Time
}

func newDiscardQueue(ivl, lifespan time.Duration) *discardQueue {
	epoch := time.Now()

	q := &discardQueue{
		expired: list.New(),
		timers:  make([]*list.List, lifespan/ivl+1),
		ivl:     ivl,
		epoch:   epoch,
		now:     epoch,
	}

	for i := range q.timers {
		q.timers[i] = list.New()
	}

	q.Poller = NewPoller(ivl, q.tick, nil)

	return q
}

type discardQueue struct {
	lock    sync.Mutex
	expired *list.List
	timers  []*list.List
	ivl     time.Duration
	epoch   time.Time
	now     time.Time
	*Poller
}

func (q *discardQueue) tick(now time.Time) {
	q.lock.Lock()
	defer q.lock.Unlock()

	q.now = now
	expired := q.timers[int(now.Sub(q.epoch)/q.ivl)%len(q.timers)]
	q.expired.PushBackList(expired)
	expired.Init()
}

func (q *discardQueue) Push(i discardQueueItem) {
	q.lock.Lock()
	defer q.lock.Unlock()

	q.push(i)
}

func (q *discardQueue) push(i discardQueueItem) {
	q.timers[int(i.Deadline().Sub(q.epoch)/q.ivl)%len(q.timers)].PushBack(i)
}

func (q *discardQueue) Pop() discardQueueItem {
	q.lock.Lock()
	defer q.lock.Unlock()

	for {
		if q.expired.Len() == 0 {
			return nil
		}

		i := q.expired.Remove(q.expired.Front()).(discardQueueItem)
		if q.now.After(i.Deadline()) {
			return i
		}
		q.push(i)
	}
}
