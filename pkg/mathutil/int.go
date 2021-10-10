package mathutil

import "math"

func MinInt(ns ...int) int {
	n := math.MaxInt
	for i := range ns {
		if ns[i] < n {
			n = ns[i]
		}
	}
	return n
}
