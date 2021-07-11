package graph

import (
	"math/rand"
	"testing"

	"github.com/tj/assert"
)

func TestMinCostMaxFlow(t *testing.T) {
	g := New(6)

	g.AddEdge(0, 1, 1, 0)
	g.AddEdge(0, 2, 1, 0)

	g.AddEdge(1, 3, 1, 1)
	g.AddEdge(1, 4, 1, 1)
	g.AddEdge(2, 3, 1, 1)

	g.AddEdge(3, 5, 1, 0)
	g.AddEdge(4, 5, 1, 0)

	var f MinCostMaxFlow
	flow, cost := f.ComputeMaxFlow(g, 0, 5)
	assert.Equal(t, 2, flow)
	assert.Equal(t, 2, cost)
}

func BenchmarkMinCostMaxFlow(b *testing.B) {
	for i := 0; i < b.N; i++ {
		g := New(6)

		g.AddEdge(0, 1, 1, 0)
		g.AddEdge(0, 2, 1, 0)

		g.AddEdge(1, 3, 1, 1)
		g.AddEdge(1, 4, 1, 1)
		g.AddEdge(2, 3, 1, 1)

		g.AddEdge(3, 5, 1, 0)
		g.AddEdge(4, 5, 1, 0)

		var f MinCostMaxFlow
		f.ComputeMaxFlow(g, 0, 5)
	}
}

func TestMinCostMaxFlowLarge(t *testing.T) {
	g := New(201)

	for i := int64(1); i < 100; i++ {
		g.AddEdge(0, i, 1, 0)
	}
	for i := int64(100); i < 200; i++ {
		g.AddEdge(i, 200, 1, 0)
	}

	r := rand.New(rand.NewSource(0))
	for i := int64(1); i < 100; i++ {
		for j := int64(100); j < 200; j++ {
			if r.Float32() < 0.1 {
				g.AddEdge(i, j, 1, 1)
			}
		}
	}

	var f MinCostMaxFlow
	flow, cost := f.ComputeMaxFlow(g, 0, 200)
	assert.Equal(t, 99, flow)
	assert.Equal(t, 99, cost)
}
