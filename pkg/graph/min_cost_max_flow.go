package graph

import "math"

const inf = int64(math.MaxInt64/2 - 1)

// MinCostMaxFlow finds the minimum cost max flow of a weighted flow network
// using the Ford–Fulkerson algorithm with Bellman-Ford search.
// SEE https://en.wikipedia.org/wiki/Ford%E2%80%93Fulkerson_algorithm
//     https://en.wikipedia.org/wiki/Bellman%E2%80%93Ford_algorithm
type MinCostMaxFlow struct {
	found           []bool
	n               int64
	cap, flow, cost []int64
	prev, dist, pi  []int64
}

func (f *MinCostMaxFlow) search(s, t int64) bool {
	for i := range f.found {
		f.found[i] = false
	}
	for i := range f.dist {
		f.dist[i] = inf
	}

	f.dist[s] = 0

	for s != f.n {
		best := f.n
		f.found[s] = true

		for i := int64(0); i < f.n; i++ {
			if f.found[i] {
				continue
			}

			if f.flow[i*f.n+s] != 0 {
				val := f.dist[s] + f.pi[s] - f.pi[i] - f.cost[i*f.n+s]
				if f.dist[i] > val {
					f.dist[i] = val
					f.prev[i] = s
				}
			}

			if f.flow[s*f.n+i] < f.cap[s*f.n+i] {
				val := f.dist[s] + f.pi[s] - f.pi[i] + f.cost[s*f.n+i]
				if f.dist[i] > val {
					f.dist[i] = val
					f.prev[i] = s
				}
			}

			if f.dist[i] < f.dist[best] {
				best = i
			}
		}

		s = best
	}

	for i := int64(0); i < f.n; i++ {
		pi := f.pi[i] + f.dist[i]
		if pi > inf {
			pi = inf
		}
		f.pi[i] = pi
	}

	return f.found[t]
}

func (f *MinCostMaxFlow) Flow(s, t int64) int64 {
	return f.flow[s*f.n+t]
}

func (f *MinCostMaxFlow) ComputeMaxFlow(g Graph, s, t int64) (flow, cost int64) {
	f.cap = g.cap
	f.cost = g.cost
	f.n = g.n

	f.found = make([]bool, f.n)
	f.flow = make([]int64, f.n*f.n)
	f.dist = make([]int64, f.n+1)
	f.prev = make([]int64, f.n)
	f.pi = make([]int64, f.n)

	for f.search(s, t) {
		pathFlow := inf
		for u := t; u != s; u = f.prev[u] {
			var pf int64
			if f.flow[u*f.n+f.prev[u]] != 0 {
				pf = f.flow[u*f.n+f.prev[u]]
			} else {
				pf = f.cap[f.prev[u]*f.n+u] - f.flow[f.prev[u]*f.n+u]
			}
			if pf < pathFlow {
				pathFlow = pf
			}
		}

		for u := t; u != s; u = f.prev[u] {
			if f.flow[u*f.n+f.prev[u]] != 0 {
				f.flow[u*f.n+f.prev[u]] -= pathFlow
				cost -= pathFlow * f.cost[u*f.n+f.prev[u]]
			} else {
				f.flow[f.prev[u]*f.n+u] += pathFlow
				cost += pathFlow * f.cost[f.prev[u]*f.n+u]
			}
		}
		flow += pathFlow
	}

	return flow, cost
}
