// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package store

import (
	"testing"

	"github.com/stretchr/testify/assert"
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

func TestPubSubReset(t *testing.T) {
	s0 := &testSubscriber{}
	p := NewPubSub(s0)

	p.Reset()

	assert.True(t, s0.reset, "subscriber should be marked reset")
}

type testSubscriber struct {
	reset  bool
	chunks []Chunk
}

func (s *testSubscriber) Reset() {
	s.reset = true
}

func (s *testSubscriber) Consume(c Chunk) {
	s.chunks = append(s.chunks, c)
}
