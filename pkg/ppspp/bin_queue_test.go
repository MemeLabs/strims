// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"testing"

	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/stretchr/testify/assert"
)

func TestHeapQueue(t *testing.T) {
	q := &binQueue{}

	for i := 0; i <= 40; i++ {
		q.Push(binmap.Bin(i), timeutil.Time(i))
	}

	var b binmap.Bin
	for i := 20; i <= 40; i += 20 {
		for it := q.IterateLessThan(timeutil.Time(i)); it.Next(); {
			assert.Equal(t, b, it.Value())
			b++
		}
		assert.Equal(t, binmap.Bin(i+1), b)
	}

	it := q.IterateLessThan(100)
	assert.False(t, it.Next())
	assert.Equal(t, binmap.None, it.Value())
}

func BenchmarkHeapQueue(b *testing.B) {
	q := &binQueue{}

	for i := 0; i <= b.N; i++ {
		if i%10 == 0 {
			for it := q.IterateLessThan(timeutil.Time(i)); it.Next(); {
			}
		}
		q.Push(0, timeutil.Time(i))
	}
}
