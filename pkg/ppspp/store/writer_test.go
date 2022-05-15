// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package store

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestWriter(t *testing.T) {
	chunkSize := 1024

	p := &testPublisher{}
	w := NewWriter(p, chunkSize)

	_, err := w.Write(make([]byte, chunkSize*3))
	assert.Nil(t, err)
	w.Flush()

	assert.Equal(t, 3, len(p.chunks), "publisher did not receive chunk")
}

type testPublisher struct {
	chunks []Chunk
}

func (s *testPublisher) Publish(c Chunk) {
	s.chunks = append(s.chunks, c)
}
