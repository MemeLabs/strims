package ppspp

import (
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/queue"
)

// binTimeoutQueueItem ...
type binTimeoutQueueItem struct {
	Time time.Time
	Bin  binmap.Bin
}

type binTimeoutQueue struct {
	ring   queue.Ring
	values []binTimeoutQueueItem
}

func newBinTimeoutQueue(size uint64) (r *binTimeoutQueue) {
	r = &binTimeoutQueue{}
	r.resize(size)
	return
}

func (s *binTimeoutQueue) grow() {
	s.resize(s.ring.Size() * 2)
}

func (s *binTimeoutQueue) resize(size uint64) {
	v := s.values
	s.values = make([]binTimeoutQueueItem, size)

	oldTail, ok := s.ring.Tail()
	s.ring.Resize(size)
	newTail, _ := s.ring.Tail()

	if ok {
		copyBinTimeoutQueue(
			[][]binTimeoutQueueItem{s.values[newTail:], s.values[:newTail]},
			[][]binTimeoutQueueItem{v[oldTail:], v[:oldTail]},
		)
	}
}

func (s *binTimeoutQueue) Push(b binmap.Bin, t time.Time) {
	i, ok := s.ring.Push()
	if !ok {
		s.grow()
		i, _ = s.ring.Push()
	}

	s.values[i] = binTimeoutQueueItem{
		Time: t,
		Bin:  b,
	}
	return
}

// Peek ...
func (s *binTimeoutQueue) Peek() (r *binTimeoutQueueItem, ok bool) {
	i, ok := s.ring.Tail()
	if ok {
		r = &s.values[i]
	}
	return
}

// Pop ...
func (s *binTimeoutQueue) Pop() (r *binTimeoutQueueItem, ok bool) {
	if i, ok := s.ring.Pop(); ok {
		r = &s.values[i]
	}
	return
}

func (s *binTimeoutQueue) IterateUntil(t time.Time) binHistoryIterator {
	return binHistoryIterator{
		t: t,
		h: s,
	}
}

type binHistoryIterator struct {
	t time.Time
	h *binTimeoutQueue
	e *binTimeoutQueueItem
}

// Next ...
func (s *binHistoryIterator) Next() (ok bool) {
	s.e, ok = s.h.Peek()
	if !ok || s.e.Time.After(s.t) {
		return false
	}
	s.h.ring.Pop()
	return
}

func (s *binHistoryIterator) Bin() binmap.Bin {
	return s.e.Bin
}

func copyBinTimeoutQueue(dst [][]binTimeoutQueueItem, src [][]binTimeoutQueueItem) (n int) {
	var i, in int
	for _, b := range src {
		var bn int
		for i < len(dst) {
			cn := copy(dst[i][in:], b[bn:])
			in += cn
			bn += cn
			n += cn

			if in == len(dst[i]) {
				in = 0
				i++
			}
			if bn == len(b) {
				break
			}
		}
	}
	return
}
