package ppspp

import (
	"math/rand"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestPeerStreamAssigner(t *testing.T) {
	caps := []int{0, 2, 0, 3, 1, 1, 0, 0, 2, 0}
	asn := newPeerStreamAssigner(32, caps)

	r := rand.New(rand.NewSource(0))
	for i := 0; i < 32; i++ {
		for j := 0; j < 10; j++ {
			if r.Float32() < 0.1 {
				asn.addCandidate(i, j)
			}
		}
	}

	actual := asn.run()

	// specific assignments aren't important only that they're stable
	expected := []peerStreamAssignment{{0, 3}, {4, 5}, {5, 4}, {10, 8}, {11, 3}, {16, 1}, {18, 3}, {25, 1}, {31, 8}}
	assert.EqualValues(t, expected, actual, "expected assignments to be stable")

	assert.LessOrEqual(t, 9, len(actual), "fewer than expected assignments")

	for _, a := range actual {
		caps[a.channel]--
		assert.LessOrEqual(t, 0, caps[a.channel], "expected assignment not to exceed channel capacity")
	}
}

func TestEdmondsKarpGraph(t *testing.T) {
	g := newEdmondsKarpGraph(6)

	g.addEdge(0, 1, 1)
	g.addEdge(0, 2, 1)

	g.addEdge(1, 3, 1)
	g.addEdge(1, 4, 1)
	g.addEdge(2, 3, 1)

	g.addEdge(3, 5, 1)
	g.addEdge(4, 5, 1)

	assert.Equal(t, 2, g.run(0, 5))
}

func TestEdmondsKarpGraphLarge(t *testing.T) {
	g := newEdmondsKarpGraph(201)

	for i := 1; i < 100; i++ {
		g.addEdge(0, i, 1)
	}
	for i := 100; i < 200; i++ {
		g.addEdge(i, 200, 1)
	}

	r := rand.New(rand.NewSource(0))
	for i := 1; i < 100; i++ {
		for j := 100; j < 200; j++ {
			if r.Float32() < 0.1 {
				g.addEdge(i, j, 1)
			}
		}
	}

	assert.Equal(t, 99, g.run(0, 200))
}
