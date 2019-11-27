package dht

import (
	"testing"
)

type node ID

func (n node) ID() ID {
	return ID(n)
}

// func TestKBucket(t *testing.T) {
// 	id := ID{0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff}
// 	b := NewKBucket(node(id), 20)

// 	for i := 0; i < 100; i++ {
// 		id, _ := NewID()
// 		b.Insert(node(id))
// 	}

// 	id, _ = NewID()
// 	v := b.Closest(id, 5)
// 	for _, vv := range v {
// 		log.Println(vv.ID().XOr(id))
// 	}
// }

func BenchmarkKBucket(b *testing.B) {
	id := ID{0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff}
	k := NewKBucket(node(id), 20)

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
