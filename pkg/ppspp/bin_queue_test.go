package ppspp

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/stretchr/testify/assert"
)

func TestHeapQueue(t *testing.T) {
	q := &binQueue{}

	for i := 0; i <= 40; i++ {
		q.Push(binmap.Bin(i), int64(i))
	}

	var b binmap.Bin
	for i := 20; i <= 40; i += 20 {
		for it := q.IterateLessThan(int64(i)); it.Next(); {
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
			for it := q.IterateLessThan(int64(i)); it.Next(); {
			}
		}
		q.Push(0, int64(i))
	}
}
