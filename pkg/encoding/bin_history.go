package encoding

import (
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/queue"
)

// binHistoryEntry ...
type binHistoryEntry struct {
	Time time.Time
	Bin  binmap.Bin
}

type binHistory struct {
	ring   queue.Ring
	values []binHistoryEntry
}

func newBinHistory(size uint64) (r *binHistory) {
	r = &binHistory{}
	r.resize(size)
	return
}

func (s *binHistory) grow() {
	s.resize(s.ring.Size() * 2)
}

func (s *binHistory) resize(size uint64) {
	v := s.values
	s.values = make([]binHistoryEntry, size)

	oldTail, ok := s.ring.Tail()
	s.ring.Resize(size)
	newTail, _ := s.ring.Tail()

	if ok {
		copyBinHistories(
			[][]binHistoryEntry{s.values[newTail:], s.values[:newTail]},
			[][]binHistoryEntry{v[oldTail:], v[:oldTail]},
		)
	}
}

func (s *binHistory) Push(b binmap.Bin) {
	i, ok := s.ring.Push()
	if !ok {
		s.grow()
		i, _ = s.ring.Push()
	}

	s.values[i] = binHistoryEntry{
		Time: time.Now(),
		Bin:  b,
	}
	return
}

// Peek ...
func (s *binHistory) Peek() (r *binHistoryEntry, ok bool) {
	i, ok := s.ring.Tail()
	if ok {
		r = &s.values[i]
	}
	return
}

// Pop ...
func (s *binHistory) Pop() (r *binHistoryEntry, ok bool) {
	if i, ok := s.ring.Pop(); ok {
		r = &s.values[i]
	}
	return
}

func (s *binHistory) IterateUntil(t time.Time) binHistoryIterator {
	return binHistoryIterator{
		t: t,
		h: s,
	}
}

type binHistoryIterator struct {
	t time.Time
	h *binHistory
	e *binHistoryEntry
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

func copyBinHistories(dst [][]binHistoryEntry, src [][]binHistoryEntry) (n int) {
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
