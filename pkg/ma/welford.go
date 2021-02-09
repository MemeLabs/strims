package ma

import "math"

// Welford implements Welford's online algorithm for variance
// SEE: https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm
type Welford struct {
	count float64
	mean  float64
	m2    float64
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
func (w *Welford) Value() (mean, variance, sampleVariance float64) {
	if w.count < 2 {
		return w.mean, 0, 0
	}
	return w.mean, w.m2 / w.count, w.m2 / (w.count - 1)
}

// Mean ...
func (w *Welford) Mean() float64 {
	return w.mean
}

// Variance ...
func (w *Welford) Variance() float64 {
	return w.m2 / (w.count - 1)
}

// StdDev ...
func (w *Welford) StdDev() float64 {
	return math.Sqrt(w.Variance())
}
