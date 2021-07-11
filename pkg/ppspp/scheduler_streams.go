package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/graph"
)

func newPeerStreamAssigner(streamCount int64, channelCaps []int64) *peerStreamAssigner {
	size := streamCount + int64(len(channelCaps)) + 2
	g := graph.New(size)

	for i := int64(0); i < streamCount; i++ {
		g.AddEdge(size-2, i, 1, 0)
	}
	for i, c := range channelCaps {
		g.AddEdge(streamCount+int64(i), size-1, c, 0)
	}

	return &peerStreamAssigner{g, streamCount, size}
}

type peerStreamAssigner struct {
	graph       graph.Graph
	streamCount int64
	size        int64
}

func (a *peerStreamAssigner) addCandidate(stream, channel, cost int64) {
	a.graph.AddEdge(stream, a.streamCount+channel, 1, cost)
}

type peerStreamAssignment struct {
	stream, channel int64
}

func (a *peerStreamAssigner) run() ([]int64, []peerStreamAssignment) {
	var f graph.MinCostMaxFlow
	f.ComputeMaxFlow(a.graph, a.size-2, a.size-1)

	var unassigned []int64
	for i := int64(0); i < a.streamCount; i++ {
		if f.Flow(a.size-2, i) == 0 {
			unassigned = append(unassigned, i)
		}
	}

	var assignments []peerStreamAssignment
	for i := int64(0); i < a.streamCount; i++ {
		for j := a.streamCount; j < a.size-2; j++ {
			if f.Flow(i, j) != 0 {
				assignments = append(assignments, peerStreamAssignment{i, j - a.streamCount})
			}
		}
	}
	return unassigned, assignments
}
