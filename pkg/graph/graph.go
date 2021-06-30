package graph

func New(n int) Graph {
	cap := make([]int, n*n)
	cost := make([]int, n*n)
	return Graph{n, cap, cost}
}

type Graph struct {
	n         int
	cap, cost []int
}

func (g *Graph) AddEdge(s, t, cap, cost int) {
	g.cap[s*g.n+t] = cap
	g.cap[t*g.n+s] = cap
	g.cost[s*g.n+t] = cost
	g.cost[t*g.n+s] = cost
}
