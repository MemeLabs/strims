package store

import (
	"testing"

	"github.com/tj/assert"
)

func TestPubSubPublish(t *testing.T) {
	s0 := &testSubscriber{}
	s1 := &testSubscriber{}
	p := NewPubSub(s0)
	p.Subscribe(s1)

	c := Chunk{
		Bin:  123,
		Data: []byte{1, 2, 3},
	}
	p.Publish(c)

	assert.Equal(t, c, s0.chunks[0], "constructor subscriber did not receive chunk")
	assert.Equal(t, c, s1.chunks[0], "subscribed subscriber did not receive chunk")
}

func TestPubSubUnsubscribe(t *testing.T) {
	s0 := &testSubscriber{}
	s1 := &testSubscriber{}
	p := NewPubSub(s0, s1)

	p.Unsubscribe(s1)

	c := Chunk{
		Bin:  123,
		Data: []byte{1, 2, 3},
	}
	p.Publish(c)

	assert.Equal(t, c, s0.chunks[0], "unmodified subscriber did not receive chunk")
	assert.Equal(t, 0, len(s1.chunks), "unsubscribed subscriber received chunk")
}

type testSubscriber struct {
	chunks []Chunk
}

func (s *testSubscriber) Consume(c Chunk) {
	s.chunks = append(s.chunks, c)
}
