package kademlia

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

type node ID

func (n node) ID() ID {
	return ID(n)
}

func TestKBucket(t *testing.T) {
	id := ID{0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff}
	b := NewKBucket(id, 20)

	for i := 0; i < 100; i++ {
		id, _ := NewID()
		b.Insert(node(id))
	}

	id, _ = NewID()
	ids := make([]Interface, 20)
	n := b.Closest(id, ids)
	for i := 1; i < n; i++ {
		assert.True(t, ids[i-1].ID().XOr(id).Less(ids[i].ID().XOr(id)))
	}
}

func BenchmarkKBucket(b *testing.B) {
	id := ID{0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff}
	k := NewKBucket(id, 20)

	for i := 0; i < 100; i++ {
		id, _ := NewID()
		k.Insert(node(id))
	}

	ids := make([]ID, 100)
	for i := 0; i < 100; i++ {
		ids[i], _ = NewID()
	}

	is := make([]Interface, 5)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		k.Closest(ids[i%100], is)
	}
}
