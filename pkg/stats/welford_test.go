package stats

import (
	"fmt"
	"math"
	"math/rand"
	"testing"

	"github.com/stretchr/testify/assert"
	"gonum.org/v1/gonum/stat"
)

const e = 1e-10

func TestWelford(t *testing.T) {
	n := 1000
	var w Welford
	vs := make([]float64, 0, n)

	for i := 0; i < n; i++ {
		v := 100 + rand.Float64()*50
		vs = append(vs, v)
		w.Update(v)
	}

	mean, variance := stat.MeanVariance(vs, nil)

	assert.LessOrEqual(t, math.Abs(w.Mean()-mean), e, "mean should be within margin of error")
	assert.LessOrEqual(t, math.Abs(w.Variance()-variance), e, "variance should be within margin of error")
}

func TestWelfordMerge(t *testing.T) {
	n := 1000
	ws := make([]Welford, 4)
	vs := make([]float64, 0, n)

	for i := 0; i < len(ws); i++ {
		for j := 0; j < n/len(ws); j++ {
			v := float64((i + 1) * j)
			vs = append(vs, v)
			ws[i].Update(v)
		}
	}

	w := WelfordMerge(ws...)

	mean, variance := stat.MeanVariance(vs, nil)

	assert.LessOrEqual(t, math.Abs(w.Mean()-mean), e, "mean should be within margin of error")
	assert.LessOrEqual(t, math.Abs(w.Variance()-variance), e, "variance should be within margin of error")
}

func TestWelfordMergeEmpty(t *testing.T) {
	ws := make([]Welford, 4)
	w := WelfordMerge(ws...)
	assert.EqualValues(t, 0, w.Count())
	assert.EqualValues(t, 0, w.Mean())
	assert.EqualValues(t, 0, w.Variance())
}

var BenchmarkWelfordMergeRes Welford

func BenchmarkWelfordMerge(b *testing.B) {
	cases := []struct {
		n  int
		wn int
	}{
		{1000, 4},
		{1000, 5},
		{1000, 10},
		{1000, 25},
		{1000, 50},
	}

	for _, c := range cases {
		c := c
		b.Run(fmt.Sprintf("samples %d chunks %d", c.n, c.wn), func(b *testing.B) {
			ws := make([]Welford, c.wn)

			for i := 0; i < len(ws); i++ {
				for j := 0; j < c.n/len(ws); j++ {
					v := float64((i + 1) * j)
					ws[i].Update(v)
				}
			}

			var res Welford

			b.ResetTimer()

			for i := 0; i < b.N; i++ {
				res = WelfordMerge(ws...)
			}

			BenchmarkWelfordMergeRes = res
		})
	}
}
