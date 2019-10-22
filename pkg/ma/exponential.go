package ma

// NewExponential ...
func NewExponential(alpha float64) Exponential {
	return Exponential{
		alpha:  alpha,
		weight: 1,
	}
}

// Exponential ...
type Exponential struct {
	value  float64
	alpha  float64
	weight float64
}

// Value ...
func (a *Exponential) Value() float64 {
	if a.weight == 1 {
		return 0
	}
	return a.value / (1 - a.weight)
}

// Set ...
func (a *Exponential) Set(v float64) {
	a.value = v
	a.weight = 0
}

// Empty ...
func (a *Exponential) Empty() bool {
	return a.weight == 1
}

// Update ...
func (a *Exponential) Update(v float64) {
	a.value = a.alpha*v + (1-a.alpha)*a.value
	a.weight *= a.alpha
}
