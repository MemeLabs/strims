package stats

import (
	"math"
)

// Welford implements Welford's online algorithm for variance
// SEE: https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm
type Welford struct {
	count float64
	mean  float64
	m2    float64
}

// Reset ...
func (w *Welford) Reset() {
	w.count = 0
	w.mean = 0
	w.m2 = 0
}

// Update ...
func (w *Welford) Update(v float64) {
	w.count++
	d := v - w.mean
	w.mean += d / w.count
	d2 := v - w.mean
	w.m2 += d * d2
}

// Value ...
func (w Welford) Value() (mean, variance, sampleVariance float64) {
	if w.count < 2 {
		return w.mean, 0, 0
	}
	return w.mean, w.m2 / w.count, w.m2 / (w.count - 1)
}

// Count ...
func (w Welford) Count() float64 {
	return w.count
}

// Mean ...
func (w Welford) Mean() float64 {
	return w.mean
}

// Variance ...
func (w Welford) Variance() float64 {
	return w.m2 / (w.count - 1)
}

// StdDev ...
func (w Welford) StdDev() float64 {
	return math.Sqrt(w.Variance())
}

func WelfordMerge(ws ...Welford) Welford {
	c := make([]Welford, 0, len(ws))
	for _, w := range ws {
		if w.count != 0 {
			c = append(c, w)
		}
	}
	if len(c) == 0 {
		return Welford{}
	}
	return welfordMerge(c...)
}

func welfordMerge(ws ...Welford) Welford {
	n := len(ws) % 2
	for i := n; i < len(ws); i += 2 {
		ws[n] = welfordMergeOne(ws[i], ws[i+1])
		n++
	}
	if n == 1 {
		return ws[0]
	}
	return welfordMerge(ws[:n]...)
}

func welfordMergeOne(w0, w1 Welford) Welford {
	count := w0.count + w1.count
	delta := w1.mean - w0.mean
	return Welford{
		count: count,
		mean:  (w0.mean*w0.count + w1.mean*w1.count) / count,
		m2:    w0.m2 + w1.m2 + delta*delta*w0.count*w1.count/count,
	}
}
