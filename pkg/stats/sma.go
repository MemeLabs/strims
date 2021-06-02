package stats

import (
	"time"
)

// NewSMA ...
func NewSMA(n int, d time.Duration) SMA {
	return SMA{
		t: time.Now(),
		d: d,
		w: make([]smaSample, n),
	}
}

// SMA ...
type SMA struct {
	t  time.Time
	d  time.Duration
	v  uint64
	n  uint64
	i  int
	wl int
	w  []smaSample
}

func (s *SMA) advance(t time.Time) {
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
func (s *SMA) Add(v uint64) {
	s.AddWithTime(v, time.Now())
}

// AddWithTime ...
func (s *SMA) AddWithTime(v uint64, t time.Time) {
	s.advance(t)

	s.w[s.i].v += v
	s.w[s.i].n++
	s.v += v
	s.n++
}

// AddN ...
func (s *SMA) AddN(c, v uint64) {
	s.AddNWithTime(c, v, time.Now())
}

// AddNWithTime ...
func (s *SMA) AddNWithTime(c, v uint64, t time.Time) {
	s.advance(t)

	s.w[s.i].v += c * v
	s.w[s.i].n += c
	s.v += c * v
	s.n += c
}

// Value ...
func (s *SMA) Value() uint64 {
	return s.ValueWithTime(time.Now())
}

// ValueWithTime ...
func (s *SMA) ValueWithTime(t time.Time) uint64 {
	s.advance(t)

	if s.n == 0 {
		return 0
	}
	return s.v / s.n
}

// Interval ...
func (s *SMA) Interval() time.Duration {
	return s.IntervalWithTime(time.Now())
}

// IntervalWithTime ...
func (s *SMA) IntervalWithTime(t time.Time) time.Duration {
	s.advance(t)

	if s.v == 0 {
		return 0
	}
	return time.Duration(s.wl) * s.d / time.Duration(s.v)
}

// SampleInterval ...
func (s *SMA) SampleInterval() time.Duration {
	return s.SampleIntervalWithTime(time.Now())
}

// SampleIntervalWithTime ...
func (s *SMA) SampleIntervalWithTime(t time.Time) time.Duration {
	s.advance(t)

	if s.n == 0 {
		return 0
	}
	return time.Duration(s.wl) * s.d / time.Duration(s.n)
}

// Rate ...
func (s *SMA) Rate(d time.Duration) uint64 {
	return s.RateWithTime(d, time.Now())
}

// RateWithTime ...
func (s *SMA) RateWithTime(d time.Duration, t time.Time) uint64 {
	s.advance(t)

	if s.wl == 0 {
		return 0
	}
	return s.v * uint64(d) / uint64(time.Duration(s.wl)*s.d)
}

// SampleRate ...
func (s *SMA) SampleRate(d time.Duration) uint64 {
	return s.SampleRateWithTime(d, time.Now())
}

// SampleRateWithTime ...
func (s *SMA) SampleRateWithTime(d time.Duration, t time.Time) uint64 {
	s.advance(t)

	if s.wl == 0 {
		return 0
	}
	return s.n * uint64(d) / uint64(time.Duration(s.wl)*s.d)
}

type smaSample struct {
	v uint64
	n uint64
}
