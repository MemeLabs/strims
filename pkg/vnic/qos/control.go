package qos

import (
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

// New ...
func New() *Control {
	return NewWithLimit(1 << 40) // 1TBps
}

// NewWithLimit ...
func NewWithLimit(limit uint64) *Control {
	c := &Control{
		hlb:   NewHLB(float64(limit)),
		ready: make(chan struct{}),
	}
	c.Class = &Class{
		ctrl: c,
		pfq: &pfqNode{
			pfqBase: pfqBase{
				weight: MaxWeight,
			},
			queue: &seff{},
		},
	}

	go c.run()

	return c
}

// Control ...
type Control struct {
	lock  sync.Mutex
	ready chan struct{}
	hlb   *HLB
	*Class
}

// SetRateLimit ...
func (c *Control) SetRateLimit(limit uint64) {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.hlb.SetLimit(float64(limit))
}

// Dequeue ...
func (c *Control) Dequeue() Packet {
	c.lock.Lock()
	defer c.lock.Unlock()

	p := c.pfq.Head()
	if p == nil || !c.hlb.Check(float64(p.Size())) {
		return nil
	}

	c.pfq.Dequeue()
	return p
}

func (c *Control) run() {
	t := timeutil.DefaultTickEmitter.DefaultTicker()
	defer t.Stop()

	for range c.ready {
		now := timeutil.Now()
		for {
			c.lock.Lock()

			p := c.pfq.Head()
			if p == nil {
				c.lock.Unlock()
				break
			}
			if !c.hlb.CheckWithTime(float64(p.Size()), now) {
				c.lock.Unlock()
				now = <-t.C
				continue
			}

			c.pfq.Dequeue()
			c.lock.Unlock()

			p.Send()
		}
	}
}

// Class ...
type Class struct {
	ctrl *Control
	pfq  *pfqNode
}

// SetWeight ...
func (c *Class) SetWeight(w uint64) {
	c.ctrl.lock.Lock()
	defer c.ctrl.lock.Unlock()
	c.pfq.weight = computeWeight(w)
}

// AddClass ...
func (c *Class) AddClass(w uint64) *Class {
	return &Class{
		ctrl: c.ctrl,
		pfq: &pfqNode{
			pfqBase: pfqBase{
				parent: c.pfq,
				weight: computeWeight(w),
			},
			queue: &seff{},
		},
	}
}

// AddSession ...
func (c *Class) AddSession(w uint64) *Session {
	return &Session{
		ctrl: c.ctrl,
		pfq: &pfqLeaf{
			pfqBase: pfqBase{
				parent: c.pfq,
				weight: computeWeight(w),
			},
			queue: &listPacketQueue{},
		},
	}
}

// Session ...
type Session struct {
	ctrl *Control
	pfq  *pfqLeaf
}

// SetWeight ...
func (s *Session) SetWeight(w uint64) {
	s.ctrl.lock.Lock()
	defer s.ctrl.lock.Unlock()
	s.pfq.weight = computeWeight(w)
}

// Close ...
func (s *Session) Close() {
	s.ctrl.lock.Lock()
	defer s.ctrl.lock.Unlock()
	s.pfq.Close()
}

// Enqueue ...
func (s *Session) Enqueue(p Packet) {
	s.ctrl.lock.Lock()
	s.pfq.Arrive(p)
	s.ctrl.lock.Unlock()

	select {
	case s.ctrl.ready <- struct{}{}:
	default:
	}
}

func computeWeight(w uint64) uint64 {
	if w > MaxWeight {
		return 1
	}
	return MaxWeight / w
}
