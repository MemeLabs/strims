// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package stats

// NewEMA ...
func NewEMA(alpha float64) EMA {
	return EMA{
		alpha:  alpha,
		weight: 1,
	}
}

// EMA ...
type EMA struct {
	value  float64
	alpha  float64
	weight float64
}

// Value ...
func (a *EMA) Value() float64 {
	if a.weight == 1 {
		return 0
	}
	return a.value / (1 - a.weight)
}

// Set ...
func (a *EMA) Set(v float64) {
	a.value = v
	a.weight = 0
}

// Empty ...
func (a *EMA) Empty() bool {
	return a.weight == 1
}

// Update ...
func (a *EMA) Update(v float64) {
	a.value = a.alpha*v + (1-a.alpha)*a.value
	a.weight *= a.alpha
}
