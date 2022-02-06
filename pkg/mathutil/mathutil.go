package mathutil

import "math"

// LogisticFunc
// SEE: https://en.wikipedia.org/wiki/Logistic_function
func LogisticFunc(x0, L, k float64) func(x float64) float64 {
	return func(x float64) float64 {
		return L / (1.0 + math.Pow(math.E, (-k*(x-x0))))
	}
}
