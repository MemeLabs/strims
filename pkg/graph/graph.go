package graph

func New(n int) Graph {
	cap := make([][]int, n)
	for i := 0; i < n; i++ {
		cap[i] = make([]int, n)
	}
	cost := make([][]int, n)
	for i := 0; i < n; i++ {
		cost[i] = make([]int, n)
	}

	return Graph{cap, cost}
}

type Graph struct {
	cap, cost [][]int
}

func (g *Graph) AddEdge(s, t, cap, cost int) {
	g.cap[s][t] = cap
	g.cap[t][s] = cap
	g.cost[s][t] = cost
	g.cost[t][s] = cost
}
