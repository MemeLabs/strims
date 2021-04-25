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

func BenchmarkPool(b *testing.B) {
	p := New(8)

	p.Put(p.Get(1024))

	b.ResetTimer()

	for n := 0; n < b.N; n++ {
		p.Put(p.Get(1024))
	}
}
