package ppspp

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestPeerWriterQueuePushPop(t *testing.T) {
	n := 10
	writers := make([]PeerWriter, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		w := &mockPeerWriter{ID: i}
		writers[i] = w
		q.Push(w)
	}

	assert.False(t, q.Empty())
	dq := q.Detach()
	assert.True(t, q.Empty())
	for i := 0; i < 10; i++ {
		w, ok := dq.Pop()
		assert.Equal(t, i, w.(*mockPeerWriter).ID)
		assert.True(t, ok)
	}
}

func TestPeerWriterQueueDeduplicate(t *testing.T) {
	n := 10
	writers := make([]PeerWriter, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		w := &mockPeerWriter{ID: i}
		writers[i] = w
		q.Push(w)
		q.Push(w)
	}

	assert.False(t, q.Empty())
	dq := q.Detach()
	assert.True(t, q.Empty())
	for i := 0; i < 10; i++ {
		w, ok := dq.Pop()
		assert.Equal(t, i, w.(*mockPeerWriter).ID)
		assert.True(t, ok)
	}
}

func TestPeerWriterQueueRemove(t *testing.T) {
	n := 10
	writers := make([]PeerWriter, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		w := &mockPeerWriter{ID: i}
		writers[i] = w
		q.Push(w)
	}

	q.Remove(writers[0])
	q.Remove(writers[9])

	assert.False(t, q.Empty())
	dq := q.Detach()
	assert.True(t, q.Empty())
	for i := 1; i < 9; i++ {
		w, ok := dq.Pop()
		assert.Equal(t, i, w.(*mockPeerWriter).ID)
		assert.True(t, ok)
	}
}

var BenchmarkPeerWriterQueueResult bool

func BenchmarkPeerWriterQueue(b *testing.B) {
	n := 10
	writers := make([]PeerWriter, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		writers[i] = &mockPeerWriter{ID: i}
	}

	b.ResetTimer()

	var res bool

	for n := 0; n < b.N; n++ {
		for _, w := range writers {
			q.Push(w)
		}

		dq := q.Detach()
		for i := 0; i < 10; i++ {
			_, res = dq.Pop()
		}
	}

	BenchmarkPeerWriterQueueResult = res
}
