package sortutil

import (
	"math/rand"
	"sort"
	"testing"
)

func BenchmarkGenericUint64(b *testing.B) {
	src := make([]uint64, 100)
	for i := range src {
		src[i] = uint64(i)
	}
	rand.New(rand.NewSource(1234)).Shuffle(len(src), func(i, j int) { src[i], src[j] = src[j], src[i] })

	vs := make([]uint64, len(src))
	copy(vs, src)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		sort.Sort(OrderedSlice[uint64](vs))
	}
}

func BenchmarkUint64(b *testing.B) {
	src := make([]uint64, 100)
	for i := range src {
		src[i] = uint64(i)
	}
	rand.New(rand.NewSource(1234)).Shuffle(len(src), func(i, j int) { src[i], src[j] = src[j], src[i] })

	vs := make([]uint64, len(src))
	copy(vs, src)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		sort.Sort(Uint64Slice(vs))
	}
}
