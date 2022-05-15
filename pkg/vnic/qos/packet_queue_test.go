// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package qos

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestListPacketQueue(t *testing.T) {
	var q listPacketQueue

	expected := make([]Packet, 10)
	for i := range expected {
		p := &noopPacket{}
		expected[i] = p
		q.Enqueue(p)
	}

	var actual []Packet
	for {
		p := q.Dequeue()
		if p == nil {
			break
		}
		actual = append(actual, p)
	}

	assert.Equal(t, expected, actual)
}
