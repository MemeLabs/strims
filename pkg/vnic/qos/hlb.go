package qos

import (
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

// NewHLBWithParent ...
func NewHLBWithParent(limit float64, parent *HLB) *HLB {
	m := &HLB{
		parent:   parent,
		lastTick: timeutil.Now(),
	}
	m.SetLimit(limit)
	return m
}

// NewHLB ...
func NewHLB(limit float64) *HLB {
	return NewHLBWithParent(limit, nil)
}

// HLB ...
type HLB struct {
	parent   *HLB
	limit    float64
	rate     float64
	value    float64
	lastTick timeutil.Time
}

// SetLimit ...
func (m *HLB) SetLimit(limit float64) {
	m.limit = limit
	m.rate = limit / float64(time.Second)
	if m.value > limit {
		m.value = limit
	}
}

// Check ...
func (m *HLB) Check(n float64) bool {
	return m.CheckWithTime(n, timeutil.Now())
}

// CheckWithTime ...
func (m *HLB) CheckWithTime(n float64, t timeutil.Time) bool {
	d := float64(t.Sub(m.lastTick))
	m.lastTick = t
	m.value -= d * m.rate
	if m.value < 0 {
		m.value = 0
	}

	if m.value+n >= m.limit || (m.parent != nil && !m.parent.CheckWithTime(n, t)) {
		return false
	}

	m.value += n
	return true
}
