// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package stats

import (
	"log"
	"math/rand"
	"testing"
)

func TestWelchTTest(t *testing.T) {
	var cases = []struct {
		A, B []float64
	}{
		{
			A: []float64{27.5, 21.0, 19.0, 23.6, 17.0, 17.9, 16.9, 20.1, 21.9, 22.6, 23.1, 19.6, 19.0, 21.7, 21.4},
			B: []float64{27.1, 22.0, 20.8, 23.4, 23.4, 23.5, 25.8, 22.0, 24.8, 20.2, 21.9, 22.1, 22.9, 20.5, 24.4},
		},
		{
			A: []float64{17.2, 20.9, 22.6, 18.1, 21.7, 21.4, 23.5, 24.2, 14.7, 21.8},
			B: []float64{21.5, 22.8, 21.0, 23.0, 21.6, 23.6, 22.5, 20.7, 23.4, 21.8, 20.7, 21.7, 21.5, 22.5, 23.6, 21.5, 22.5, 23.5, 21.5, 21.8},
		},
		{
			A: []float64{19.8, 20.4, 19.6, 17.8, 18.5, 18.9, 18.3, 18.9, 19.5, 22.0},
			B: []float64{28.2, 26.6, 20.1, 23.3, 25.2, 22.1, 17.7, 27.6, 20.6, 13.7, 23.2, 17.5, 20.6, 18.0, 23.9, 21.6, 24.3, 20.4, 24.0, 13.2},
		},
	}

	for _, c := range cases {
		t.Run("", func(t *testing.T) {
			var w0, w1 Welford
			for i := 0; i < len(c.A); i++ {
				w0.Update(c.A[i])
			}
			for i := 0; i < len(c.B); i++ {
				w1.Update(c.B[i])
			}

			log.Println("mean", w0.Mean(), w1.Mean())
			log.Println("var", w0.Variance(), w1.Variance())
			log.Println("t", WelchTTest(w0, w1))
			log.Println("v", WelchSatterthwaite(w0, w1))
			log.Println("p", TDistribution(WelchTTest(w0, w1), WelchSatterthwaite(w0, w1)))
		})
	}
}

func TestWelchTTest2(t *testing.T) {
	var w [5]Welford
	n := 1000000
	for i := 0; i < n; i++ {
		w[0].Update(54.99 + rand.Float64()*10)
		w[1].Update(55 + rand.Float64()*10)
		w[2].Update(55 + rand.Float64()*10)
		w[3].Update(55 + rand.Float64()*10)
		w[4].Update(55 + rand.Float64()*10)
	}

	g := WelfordMerge(w[:]...)

	log.Println("count", w[0].Count(), g.Count())
	log.Println("mean", w[0].Mean(), g.Mean())
	log.Println("var", w[0].Variance(), g.Variance())
	log.Println("t", WelchTTest(w[0], g))
	log.Println("v", WelchSatterthwaite(w[0], g))
	log.Println("p", TDistribution(WelchTTest(w[0], g), WelchSatterthwaite(w[0], g)))
}
