package qos

import (
	"math"
	"sync"
)

// SEFF (Smallest Eligible virtual Finish time First) queue from
//  I. Stoica and H. Abdel-Wahab, "Earliest Eligible Virtual Deadline
//  First: A Flexible and Accurate Mechanism for Proportional Share
//  Resource Allocation"
//
//  http://www.cs.berkeley.edu/~istoica/papers/eevdf-tr-95.pdf

var seffSessionPool = sync.Pool{
	New: func() interface{} {
		return &seffSession{}
	},
}

type seff struct {
	root *seffSession
}

func (q *seff) Enqueue(p PFQ) {
	s := seffSessionPool.Get().(*seffSession)
	s.init(p)
	q.root = q.insert(q.root, s)
}

func (q *seff) SelectNext(vtime uint64) PFQ {
	s := q.next(q.root, vtime)
	if s == nil {
		return nil
	}

	q.root = q.delete(q.root, s)
	defer seffSessionPool.Put(s)
	return s.PFQ
}

func (q *seff) Delete(p PFQ) {
	s := seffSessionPool.Get().(*seffSession)
	s.init(p)
	q.root = q.delete(q.root, s)
	seffSessionPool.Put(s)
}

func (q *seff) ComputeVirtualTime(vtime, work uint64) uint64 {
	var mst uint64
	if t := q.root; t != nil {
		for t.left != nil {
			t = t.left
		}
		mst = t.stime
	}
	return maxUint64(vtime+work, mst)
}

func (q *seff) insert(t, s *seffSession) *seffSession {
	if t == nil {
		return s
	}

	if t.stime < s.stime {
		t.right = q.insert(t.right, s)
	} else {
		t.left = q.insert(t.left, s)
	}

	t = q.balance(t)
	t.updateStats()
	return t
}

func (q *seff) next(t *seffSession, vtime uint64) *seffSession {
	var pathReq, stTree *seffSession

	for t != nil {
		if t.stime <= vtime {
			if pathReq == nil || pathReq.ftime > t.ftime {
				pathReq = t
			}
			if stTree == nil || (t.left != nil && stTree.minFTime > t.left.minFTime) {
				stTree = t.left
			}
			t = t.right
		} else {
			t = t.left
		}
	}

	if stTree == nil || stTree.minFTime >= pathReq.ftime {
		return pathReq
	}

	t = stTree
	for t != nil {
		if stTree.minFTime == t.ftime {
			return t
		}
		if t.minFTime == t.left.minFinish() {
			t = t.left
		} else {
			t = t.right
		}
	}

	return t
}

func (q *seff) delete(r, s *seffSession) *seffSession {
	if r == nil {
		return nil
	}

	if r.PFQ == s.PFQ {
		if s.right == nil || s.left == nil {
			c := s.right
			if s.left != nil {
				c = s.left
			}

			if c != nil {
				c = q.balance(c)
				c.updateStats()
			}
			return c
		}

		c := s.right
		for c.left != nil {
			c = c.left
		}
		s.right = q.delete(s.right, c)

		c.left = s.left
		c.right = s.right

		c = q.balance(c)
		c.updateStats()
		return c
	}

	if r.stime < s.stime {
		r.right = q.delete(r.right, s)
		r.right.updateStats()
	} else if r.stime > s.stime {
		r.left = q.delete(r.left, s)
		r.left.updateStats()
	} else {
		if r.left != nil && r.left.minFTime <= s.ftime {
			r.left = q.delete(r.left, s)
			r.left.updateStats()
		}
		if r.right != nil && r.right.minFTime <= s.ftime {
			r.right = q.delete(r.right, s)
			r.right.updateStats()
		}
	}

	r = q.balance(r)
	r.updateStats()
	return r
}

func (q *seff) balance(t *seffSession) *seffSession {
	bf := t.balanceFactor()
	if bf == 2 {
		if t.right.balanceFactor() < 0 {
			return q.rotateRightLeft(t)
		}
		return q.rotateLeft(t)
	} else if bf == -2 {
		if t.left.balanceFactor() > 0 {
			return q.rotateLeftRight(t)
		}
		return q.rotateRight(t)
	}
	return t
}

func (q *seff) rotateRightLeft(t *seffSession) *seffSession {
	t.right = q.rotateRight(t.right)
	return q.rotateLeft(t)
}

func (q *seff) rotateLeftRight(t *seffSession) *seffSession {
	t.left = q.rotateLeft(t.left)
	return q.rotateRight(t)
}

func (q *seff) rotateRight(t *seffSession) *seffSession {
	m := t.left
	t.left = m.right
	m.right = t
	t.updateStats()
	return m
}

func (q *seff) rotateLeft(t *seffSession) *seffSession {
	m := t.right
	t.right = m.left
	m.left = t
	t.updateStats()
	return m
}

type seffSession struct {
	PFQ
	stime    uint64
	ftime    uint64
	minFTime uint64
	height   int
	left     *seffSession
	right    *seffSession
}

func (s *seffSession) init(p PFQ) {
	s.PFQ = p
	s.stime = p.startTime()
	s.ftime = p.finishTime()
	s.minFTime = s.ftime
	s.height = 0
	s.left = nil
	s.right = nil
}

func (s *seffSession) updateStats() {
	if s != nil {
		s.height = s.computeHeight()
		s.minFTime = s.computeMinFinish()
	}
}

func (s *seffSession) finishTime() uint64 {
	return s.ftime
}

func (s *seffSession) computeHeight() int {
	lh := s.left.getHeight()
	rh := s.right.getHeight()
	if lh > rh {
		return lh + 1
	}
	return rh + 1
}

func (s *seffSession) getHeight() int {
	if s == nil {
		return -1
	}
	return s.height
}

func (s *seffSession) balanceFactor() int {
	return s.right.getHeight() - s.left.getHeight()
}

func (s *seffSession) computeMinFinish() uint64 {
	lf := s.left.minFinish()
	rf := s.right.minFinish()
	sf := s.ftime
	if lf < rf {
		if lf < sf {
			return lf
		}
		return sf
	}
	if sf < rf {
		return sf
	}
	return rf
}

func (s *seffSession) minFinish() uint64 {
	if s == nil {
		return math.MaxUint64
	}
	return s.minFTime
}
