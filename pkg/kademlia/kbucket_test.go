package kademlia

import (
	"log"
	"testing"
)

type node ID

func (n node) ID() ID {
	return ID(n)
}

func TestKBucket(t *testing.T) {
	id := ID{0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff}
	b := NewKBucket(id, 20)

	for i := 0; i < 100; i++ {
		id, _ := NewID()
		b.Insert(node(id))
	}

	id, _ = NewID()
	ids := make([]Interface, 5)
	n := b.Closest(id, ids)
	for i := 0; i < n; i++ {
		vv := ids[i]
		log.Println(vv.ID().XOr(id))
	}
}

func BenchmarkKBucket(b *testing.B) {
	id := ID{0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff}
	k := NewKBucket(id, 20)

	for i := 0; i < 100; i++ {
		id, _ := NewID()
		k.Insert(node(id))
	}

	ids := make([]ID, 100)
	for i := 0; i < 100; i++ {
		ids[i], _ = NewID()
	}

	is := make([]Interface, 3)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		k.Closest(ids[i%100], is)
	}
}
