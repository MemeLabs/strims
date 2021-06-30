package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/graph"
)

func newPeerStreamAssigner(streamCount int, channelCaps []int) *peerStreamAssigner {
	size := streamCount + len(channelCaps) + 2
	g := graph.New(size)

	for i := 0; i < streamCount; i++ {
		g.AddEdge(size-2, i, 1, 0)
	}
	for i, c := range channelCaps {
		g.AddEdge(streamCount+i, size-1, c, 0)
	}

	return &peerStreamAssigner{g, streamCount, size}
}

type peerStreamAssigner struct {
	graph       graph.Graph
	streamCount int
	size        int
}

func (a *peerStreamAssigner) addCandidate(stream, channel, cost int) {
	a.graph.AddEdge(stream, a.streamCount+channel, 1, cost)
}

type peerStreamAssignment struct {
	stream, channel int
}

func (a *peerStreamAssigner) run() ([]int, []peerStreamAssignment) {
	var f graph.MinCostMaxFlow
	f.ComputeMaxFlow(a.graph, a.size-2, a.size-1)
	flow := f.Flow()

	var unassigned []int
	for i := 0; i < a.streamCount; i++ {
		if flow[a.size-2][i] == 0 {
			unassigned = append(unassigned, i)
		}
	}

	var assignments []peerStreamAssignment
	for i := 0; i < a.streamCount; i++ {
		for j := a.streamCount; j < a.size-2; j++ {
			if flow[i][j] > 0 {
				assignments = append(assignments, peerStreamAssignment{i, j - a.streamCount})
			}
		}
	}
	return unassigned, assignments
}
