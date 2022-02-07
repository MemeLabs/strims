package ppspp

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/stretchr/testify/assert"
)

func TestPeerDataQueuePushPop(t *testing.T) {
	w := &mockPeerWriter{}
	var q peerDataQueue

	for i := binmap.Bin(0); i <= 8; i += 2 {
		q.Push(w, i, timeutil.EpochTime)
	}

	assert.False(t, q.Empty())
	for i := binmap.Bin(0); i <= 8; i += 2 {
		pw, pb, ts, ok := q.Pop()
		assert.Equal(t, w, pw)
		assert.Equal(t, i, pb)
		assert.Equal(t, timeutil.EpochTime, ts)
		assert.True(t, ok)
	}
	assert.True(t, q.Empty())
}

func TestPeerDataQueuePushFrontPop(t *testing.T) {
	w := &mockPeerWriter{}
	var q peerDataQueue

	for i := binmap.Bin(2); i <= 8; i += 2 {
		q.PushFront(w, i, timeutil.EpochTime)
	}

	assert.False(t, q.Empty())
	for i := binmap.Bin(8); i >= 2; i -= 2 {
		pw, pb, ts, ok := q.Pop()
		assert.Equal(t, w, pw)
		assert.Equal(t, i, pb)
		assert.Equal(t, timeutil.EpochTime, ts)
		assert.True(t, ok)
	}
	assert.True(t, q.Empty())
}

func TestPeerDataQueueRemove(t *testing.T) {
	w := &mockPeerWriter{}
	var q peerDataQueue

	for i := binmap.Bin(0); i < 16; i += 2 {
		q.Push(w, i, timeutil.EpochTime)
	}

	q.Remove(w, binmap.Bin(11))

	assert.False(t, q.Empty())
	for i := binmap.Bin(0); i < 8; i += 2 {
		pw, pb, ts, ok := q.Pop()
		assert.Equal(t, w, pw)
		assert.Equal(t, i, pb)
		assert.Equal(t, timeutil.EpochTime, ts)
		assert.True(t, ok)
	}

	assert.True(t, q.Empty())
}

func BenchmarkPeerDataQueue(b *testing.B) {
	w := &mockPeerWriter{}
	var q peerDataQueue

	size := 16

	for i := 0; i < size && i < b.N; i++ {
		q.Push(w, binmap.Bin(i*2), timeutil.EpochTime)
	}

	for i := size; i < b.N; i++ {
		q.Push(w, binmap.Bin(i*2), timeutil.EpochTime)
		q.Pop()
	}

	for !q.Empty() {
		q.Pop()
	}
}
