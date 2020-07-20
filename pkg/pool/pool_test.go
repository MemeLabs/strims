package pool

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestPool(t *testing.T) {
	p := New(8)

	b := p.Get(1024)
	assert.Equal(t, 1024, len(*b), "expected 1024 byte slice")
	p.Put(b)

	b = p.Get(1024)
	assert.Equal(t, 1024, len(*b), "expected 1024 byte slice")
}

func BenchmarkPool(b *testing.B) {
	p := New(8)

	p.Put(p.Get(1024))

	b.ResetTimer()

	for n := 0; n < b.N; n++ {
		p.Put(p.Get(1024))
	}
}
