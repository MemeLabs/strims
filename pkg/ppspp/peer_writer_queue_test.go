package ppspp

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

type writerTicket struct {
	t *PeerWriterQueueTicket
	w *mockPeerWriter
}

func TestPeerWriterQueuePushPop(t *testing.T) {
	n := 10
	tickets := make([]writerTicket, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		t := writerTicket{
			t: &PeerWriterQueueTicket{},
			w: &mockPeerWriter{ID: i},
		}
		tickets[i] = t
		q.Push(t.t, t.w)
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
	tickets := make([]writerTicket, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		t := writerTicket{
			t: &PeerWriterQueueTicket{},
			w: &mockPeerWriter{ID: i},
		}
		tickets[i] = t
		q.Push(t.t, t.w)
		q.Push(t.t, t.w)
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
	tickets := make([]writerTicket, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		t := writerTicket{
			t: &PeerWriterQueueTicket{},
			w: &mockPeerWriter{ID: i},
		}
		tickets[i] = t
		q.Push(t.t, t.w)
	}

	q.Remove(tickets[0].w)
	q.Remove(tickets[9].w)

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
	tickets := make([]writerTicket, n)
	q := newPeerWriterQueue()

	for i := 0; i < 10; i++ {
		tickets[i] = writerTicket{
			t: &PeerWriterQueueTicket{},
			w: &mockPeerWriter{ID: i},
		}
	}

	b.ResetTimer()

	var res bool

	for n := 0; n < b.N; n++ {
		for _, t := range tickets {
			q.Push(t.t, t.w)
		}

		dq := q.Detach()
		for i := 0; i < 10; i++ {
			_, res = dq.Pop()
		}
	}

	BenchmarkPeerWriterQueueResult = res
}
