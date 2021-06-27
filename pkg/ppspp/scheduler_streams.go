package ppspp

import (
	"math"
)

func newPeerStreamAssigner(streamCount int, channelCaps []int) *peerStreamAssigner {
	size := streamCount + len(channelCaps) + 2
	g := &edmondsKarpGraph{
		nodes:   make([]edmondsKarpNode, size),
		visited: make([]bool, size),
		queue:   make([]int, 0, size),
	}

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
	channel, stream int
}

func (a *peerStreamAssigner) run() []peerStreamAssignment {
	a.graph.run(a.size-2, a.size-1)

	var res []peerStreamAssignment
	for i := 0; i < a.streamCount; i++ {
		for _, e := range a.graph.nodes[i].edges {
			if e.value == 0 {
				res = append(res, peerStreamAssignment{e.node - a.streamCount, i})
			}
		}
	}
	return res
}

// Edmonds–Karp algorithm for finding maximum flow in a flow network
// SEE https://en.wikipedia.org/wiki/Edmonds%E2%80%93Karp_algorithm
type edmondsKarpGraph struct {
	nodes   []edmondsKarpNode
	visited []bool
	queue   []int
}

type edmondsKarpNode struct {
	edges []edmondsKarpEdge
	prev  edmondsKarpTrace
}

type edmondsKarpTrace struct {
	node int
	edge int
}

type edmondsKarpEdge struct {
	node  int
	value int
}

func (f *edmondsKarpGraph) addEdge(a, b, weight int) {
	f.nodes[a].edges = append(f.nodes[a].edges, edmondsKarpEdge{b, weight})
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

		for i, e := range f.nodes[u].edges {
			if !f.visited[e.node] && e.value > 0 {
				queue = append(queue, e.node)
				f.visited[e.node] = true
				f.nodes[e.node].prev = edmondsKarpTrace{u, i}
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
			if f := f.nodes[p.node].edges[p.edge].value; f < pathFlow {
				pathFlow = f
			}
			s = p.node
		}

		maxFlow += pathFlow

		v := sink
		for v != source {
			p := f.nodes[v].prev
			f.nodes[p.node].edges[p.edge].value -= pathFlow
			v = p.node
		}
	}

	return maxFlow
}
