package queue

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestRing(t *testing.T) {
	r := NewRing[int](16)
	for i := 0; i < 32; i++ {
		r.PushFront(i)
	}
	h, ok := r.Head()
	assert.Equal(t, 0, h)
	assert.True(t, ok)

	l, ok := r.Tail()
	assert.Equal(t, 31, l)
	assert.True(t, ok)

	for i := 0; i < 16; i++ {
		v, ok := r.Pop()
		assert.Equal(t, i, v)
		assert.True(t, ok)
	}

	i := 31
	for it := r.Iterator(); it.Next(); {
		assert.Equal(t, i, it.Value())
		i--
	}
}
