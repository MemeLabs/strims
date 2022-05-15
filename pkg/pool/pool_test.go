// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package pool

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestPool(t *testing.T) {
	p := New(8)

	for i := 0; i < 2; i++ {
		b := p.Get(1024)
		assert.Equal(t, 1024, len(*b), "expected buffer length 1024")
		assert.Equal(t, 1024, cap(*b), "expected buffer capacity 1024")
		p.Put(b)
	}
}

func TestPoolMaxSize(t *testing.T) {
	p := New(8)
	b := p.Get(p.MaxSize())
	assert.Equal(t, p.MaxSize(), len(*b))
}

func TestPoolGetZero(t *testing.T) {
	p := New(8)
	p.Get(0)
}

func BenchmarkPool(b *testing.B) {
	p := New(8)

	p.Put(p.Get(1024))

	b.ResetTimer()

	for n := 0; n < b.N; n++ {
		p.Put(p.Get(1024))
	}
}
