// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"math/rand"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestPeerStreamAssigner(t *testing.T) {
	caps := []int64{0, 2, 0, 3, 1, 1, 0, 0, 2, 0}
	asn := newPeerStreamAssigner(32, caps)

	r := rand.New(rand.NewSource(0))
	for i := int64(0); i < 32; i++ {
		for j := int64(0); j < 10; j++ {
			if r.Float32() < 0.1 {
				asn.addCandidate(i, j, 1)
			}
		}
	}

	_, actual := asn.run()

	// specific assignments aren't important only that they're stable
	expected := []peerStreamAssignment{{0, 3}, {4, 5}, {5, 4}, {10, 8}, {11, 3}, {16, 1}, {18, 3}, {25, 1}, {31, 8}}
	assert.EqualValues(t, expected, actual, "expected assignments to be stable")

	assert.LessOrEqual(t, 9, len(actual), "fewer than expected assignments")

	for _, a := range actual {
		caps[a.channel]--
		assert.LessOrEqual(t, int64(0), caps[a.channel], "expected assignment not to exceed channel capacity")
	}
}
