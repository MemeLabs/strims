package ppspp

import (
	"math"
)

func newPeerStreamAssigner(streamCount int, channelCaps []int) *peerStreamAssigner {
	size := streamCount + len(channelCaps) + 2
	g := newEdmondsKarpGraph(size)

	for i := 0; i < streamCount; i++ {
		g.addEdge(size-2, i, 1)
	}
	for i, c := range channelCaps {
		g.addEdge(streamCount+i, size-1, c)
	}

	return &peerStreamAssigner{g, streamCount, size}
}

type peerStreamAssigner struct {
	graph       *edmondsKarpGraph
	streamCount int
	size        int
}

func (a *peerStreamAssigner) addCandidate(stream, channel int) {
	a.graph.addEdge(stream, a.streamCount+channel, 1)
}

type peerStreamAssignment struct {
	stream, channel int
}

func (a *peerStreamAssigner) run() []peerStreamAssignment {
	a.graph.run(a.size-2, a.size-1)

	var res []peerStreamAssignment
	for i := 0; i < a.streamCount; i++ {
		for _, e := range a.graph.nodes[i].edges {
			if a.graph.edgeValue(i, e) == 0 {
				res = append(res, peerStreamAssignment{i, e - a.streamCount})
			}
		}
	}
	return res
}

// Edmonds–Karp algorithm for finding maximum flow in a flow network
// SEE https://en.wikipedia.org/wiki/Edmonds%E2%80%93Karp_algorithm
func newEdmondsKarpGraph(size int) *edmondsKarpGraph {
	return &edmondsKarpGraph{
		nodes:   make([]edmondsKarpNode, size),
		edges:   make([]int, size*size),
		visited: make([]bool, size),
		queue:   make([]int, 0, size),
	}
}

type edmondsKarpGraph struct {
	nodes   []edmondsKarpNode
	edges   []int
	visited []bool
	queue   []int
}

type edmondsKarpNode struct {
	edges []int
	prev  int
}

func (f *edmondsKarpGraph) addEdge(s, t, value int) {
	f.nodes[s].edges = append(f.nodes[s].edges, t)
	f.nodes[t].edges = append(f.nodes[t].edges, s)
	f.setEdgeValue(s, t, value)
	f.setEdgeValue(t, s, value)
}

func (f *edmondsKarpGraph) edgeValue(s, t int) int {
	return f.edges[s*len(f.nodes)+t]
}

func (f *edmondsKarpGraph) setEdgeValue(s, t, v int) {
	f.edges[s*len(f.nodes)+t] = v
}

func (f *edmondsKarpGraph) bfs(s, t int) bool {
	for i := range f.visited {
		f.visited[i] = false
	}

	queue := append(f.queue, s)
	f.visited[s] = true

	for len(queue) != 0 {
		u := queue[0]
		queue = queue[1:]

		for _, e := range f.nodes[u].edges {
			if !f.visited[e] && f.edgeValue(u, e) > 0 {
				queue = append(queue, e)
				f.visited[e] = true
				f.nodes[e].prev = u
			}
		}
	}

	return f.visited[t]
}

func (f *edmondsKarpGraph) run(source, sink int) int {
	maxFlow := 0

	for f.bfs(source, sink) {
		pathFlow := math.MaxInt
		s := sink

		for s != source {
			p := f.nodes[s].prev
			if f := f.edgeValue(p, s); f < pathFlow {
				pathFlow = f
			}
			s = p
		}

		maxFlow += pathFlow

		v := sink
		for v != source {
			u := f.nodes[v].prev
			f.setEdgeValue(u, v, f.edgeValue(u, v)-1)
			f.setEdgeValue(v, u, f.edgeValue(v, u)+1)
			v = u
		}
	}

	return maxFlow
}
