package vpn

import (
	"container/list"
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type timeoutQueueItem interface {
	Deadline() timeutil.Time
}

func newTimeoutQueue(ctx context.Context, interval, lifespan time.Duration) *timeoutQueue {
	epoch := timeutil.Now()

	q := &timeoutQueue{
		expired:  list.New(),
		windows:  make([]*list.List, lifespan/interval+1),
		interval: interval,
		epoch:    epoch,
		now:      epoch,
	}

	for i := range q.windows {
		q.windows[i] = list.New()
	}

	q.ticker = timeutil.TickerBFunc(ctx, interval, q.tick)

	return q
}

type timeoutQueue struct {
	lock     sync.Mutex
	expired  *list.List
	windows  []*list.List
	interval time.Duration
	epoch    timeutil.Time
	now      timeutil.Time
	ticker   *timeutil.TickerB
}

func (q *timeoutQueue) windowIndex(t timeutil.Time) int {
	return int(t.Sub(q.epoch)/q.interval) % len(q.windows)
}

func (q *timeoutQueue) tick(now timeutil.Time) {
	q.lock.Lock()
	defer q.lock.Unlock()

	q.now = now
	expired := q.windows[q.windowIndex(now)]
	q.expired.PushBackList(expired)
	expired.Init()
}

func (q *timeoutQueue) push(i timeoutQueueItem) {
	q.windows[q.windowIndex(i.Deadline())].PushBack(i)
}

func (q *timeoutQueue) Push(i timeoutQueueItem) {
	q.lock.Lock()
	defer q.lock.Unlock()

	q.push(i)
}

func (q *timeoutQueue) Pop() timeoutQueueItem {
	q.lock.Lock()
	defer q.lock.Unlock()

	for {
		if q.expired.Len() == 0 {
			return nil
		}

		i := q.expired.Remove(q.expired.Front()).(timeoutQueueItem)
		if q.now.After(i.Deadline()) {
			return i
		}
		q.push(i)
	}
}
