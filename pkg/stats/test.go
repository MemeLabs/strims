package stats

import (
	"math"

	"gonum.org/v1/gonum/mathext"
)

// TestSample ...
type TestSample interface {
	Count() float64
	Mean() float64
	Variance() float64
}

// WelchTTest unequal variances t-test
// SEE: https://en.wikipedia.org/wiki/Welch%27s_t-test
func WelchTTest(a, b TestSample) float64 {
	d := a.Mean() - b.Mean()
	sd := math.Sqrt(a.Variance()/a.Count() + b.Variance()/b.Count())
	return d / sd
}

// WelchSatterthwaite pooled degrees of freedom
// SEE https://en.wikipedia.org/wiki/Welch%E2%80%93Satterthwaite_equation
func WelchSatterthwaite(a, b TestSample) float64 {
	aks := a.Variance() / a.Count()
	bks := b.Variance() / b.Count()
	s := ((aks*aks)/(a.Count()-1) + (bks*bks)/(b.Count()-1))
	return ((aks + bks) * (aks + bks)) / s
}

// TDistribution student's t-distribution
// SEE https://en.wikipedia.org/wiki/Student%27s_t-distribution#Definition
func TDistribution(t, v float64) float64 {
	a := math.Sqrt(v) * mathext.Beta(0.5, v/2)
	b := math.Pow((1 + (t*t)/v), -(v+1)/2)
	return 1 / a * b
}
