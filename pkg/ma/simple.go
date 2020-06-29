package ma

import (
	"time"
)

// NewSimple ...
func NewSimple(n int, d time.Duration) Simple {
	return Simple{
		t: time.Now(),
		d: d,
		w: make([]simpleMeanSample, n),
	}
}

// Simple ...
type Simple struct {
	t  time.Time
	d  time.Duration
	v  uint64
	n  uint64
	i  int
	wl int
	w  []simpleMeanSample
}

func (s *Simple) advance(t time.Time) {
	for t.Sub(s.t) > s.d {
		s.t = s.t.Add(s.d)

		s.i++
		if s.i == len(s.w) {
			s.i = 0
		}

		if s.wl < len(s.w) {
			s.wl++
		}

		s.v -= s.w[s.i].v
		s.n -= s.w[s.i].n
		s.w[s.i].v = 0
		s.w[s.i].n = 0
	}
}

// Add ...
func (s *Simple) Add(v uint64) {
	s.AddWithTime(v, time.Now())
}

// AddWithTime ...
func (s *Simple) AddWithTime(v uint64, t time.Time) {
	s.advance(t)

	s.w[s.i].v += v
	s.w[s.i].n++
	s.v += v
	s.n++
}

// Value ...
func (s *Simple) Value() uint64 {
	return s.v / s.n
}

// Interval ...
func (s *Simple) Interval() time.Duration {
	s.advance(time.Now())

	if s.v == 0 {
		return 0
	}
	return time.Duration(s.wl) * s.d / time.Duration(s.v)
}

// SampleInterval ...
func (s *Simple) SampleInterval() time.Duration {
	s.advance(time.Now())

	if s.n == 0 {
		return 0
	}
	return time.Duration(s.wl) * s.d / time.Duration(s.n)
}

type simpleMeanSample struct {
	v uint64
	n uint64
}
