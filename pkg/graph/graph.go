// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package graph

func New(n int64) Graph {
	cap := make([]int64, n*n)
	cost := make([]int64, n*n)
	return Graph{n, cap, cost}
}

type Graph struct {
	n         int64
	cap, cost []int64
}

func (g *Graph) AddEdge(s, t, cap, cost int64) {
	g.cap[s*g.n+t] = cap
	g.cap[t*g.n+s] = cap
	g.cost[s*g.n+t] = cost
	g.cost[t*g.n+s] = cost
}
