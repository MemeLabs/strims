package stats

import (
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

// NewSMA ...
func NewSMA(n int, d time.Duration) SMA {
	return SMA{
		t: timeutil.Now(),
		d: d,
		w: make([]smaSample, n),
	}
}

// SMA ...
type SMA struct {
	t  timeutil.Time
	d  time.Duration
	v  uint64
	n  uint64
	i  int
	wl int
	w  []smaSample
}

func (s *SMA) advance(end timeutil.Time) {
	for t := s.t.Add(s.d); t < end; t = t.Add(s.d) {
		s.t = t

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

func (s *SMA) Reset() {
	s.ResetWithTime(timeutil.Now())
}

func (s *SMA) ResetWithTime(t timeutil.Time) {
	s.t = t
	s.v = 0
	s.n = 0
	s.i = 0
	s.wl = 0
	for i := range s.w {
		s.w[i].n = 0
		s.w[i].v = 0
	}
}

// Add ...
func (s *SMA) Add(v uint64) {
	s.AddWithTime(v, timeutil.Now())
}

// AddWithTime ...
func (s *SMA) AddWithTime(v uint64, t timeutil.Time) {
	s.advance(t)

	s.w[s.i].v += v
	s.w[s.i].n++
	s.v += v
	s.n++
}

// AddN ...
func (s *SMA) AddN(c, v uint64) {
	s.AddNWithTime(c, v, timeutil.Now())
}

// AddNWithTime ...
func (s *SMA) AddNWithTime(c, v uint64, t timeutil.Time) {
	s.advance(t)

	s.w[s.i].v += c * v
	s.w[s.i].n += c
	s.v += c * v
	s.n += c
}

// Value ...
func (s *SMA) Value() uint64 {
	return s.ValueWithTime(timeutil.Now())
}

// ValueWithTime ...
func (s *SMA) ValueWithTime(t timeutil.Time) uint64 {
	s.advance(t)

	if s.n == 0 {
		return 0
	}
	return s.v / s.n
}

// Interval ...
func (s *SMA) Interval() time.Duration {
	return s.IntervalWithTime(timeutil.Now())
}

// IntervalWithTime ...
func (s *SMA) IntervalWithTime(t timeutil.Time) time.Duration {
	s.advance(t)

	if s.v == 0 {
		return 0
	}
	return time.Duration(s.wl) * s.d / time.Duration(s.v)
}

// SampleInterval ...
func (s *SMA) SampleInterval() time.Duration {
	return s.SampleIntervalWithTime(timeutil.Now())
}

// SampleIntervalWithTime ...
func (s *SMA) SampleIntervalWithTime(t timeutil.Time) time.Duration {
	s.advance(t)

	if s.n == 0 {
		return 0
	}
	return time.Duration(s.wl) * s.d / time.Duration(s.n)
}

// Rate ...
func (s *SMA) Rate(d time.Duration) uint64 {
	return s.RateWithTime(d, timeutil.Now())
}

// RateWithTime ...
func (s *SMA) RateWithTime(d time.Duration, t timeutil.Time) uint64 {
	s.advance(t)

	if s.wl == 0 {
		return 0
	}
	return s.v * uint64(d) / uint64(time.Duration(s.wl)*s.d)
}

// SampleRate ...
func (s *SMA) SampleRate(d time.Duration) uint64 {
	return s.SampleRateWithTime(d, timeutil.Now())
}

// SampleRateWithTime ...
func (s *SMA) SampleRateWithTime(d time.Duration, t timeutil.Time) uint64 {
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
