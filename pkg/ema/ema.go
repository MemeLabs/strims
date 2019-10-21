package ema

// New ...
func New(alpha float64) Mean {
	return Mean{
		alpha:  alpha,
		weight: 1,
	}
}

// Mean ...
type Mean struct {
	value  float64
	alpha  float64
	weight float64
}

// Value ...
func (a *Mean) Value() float64 {
	if a.weight == 1 {
		return 0
	}
	return a.value / (1 - a.weight)
}

// Set ...
func (a *Mean) Set(v float64) {
	a.value = v
	a.weight = 0
}

// Empty ...
func (a *Mean) Empty() bool {
	return a.weight == 1
}

// Update ...
func (a *Mean) Update(v float64) {
	a.value = a.alpha*v + (1-a.alpha)*a.value
	a.weight *= a.alpha
}
