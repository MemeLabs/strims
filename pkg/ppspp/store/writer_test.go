package store

import (
	"testing"

	"github.com/tj/assert"
)

func TestWriter(t *testing.T) {
	chunkSize := 1024

	p := &testPublisher{}
	w := NewWriter(p, chunkSize)

	w.Write(make([]byte, chunkSize*3))
	w.Flush()

	assert.Equal(t, 3, len(p.chunks), "publisher did not receive chunk")
}

type testPublisher struct {
	chunks []Chunk
}

func (s *testPublisher) Publish(c Chunk) bool {
	s.chunks = append(s.chunks, c)
	return true
}
