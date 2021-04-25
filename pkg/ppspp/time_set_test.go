package ppspp

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

func BenchmarkTimeSetPrune(b *testing.B) {
	var s timeSet

	var n int
	for i := binmap.Bin(0); i < binmap.Bin(b.N*2); i += 2 {
		if n++; n >= 1024 {
			n = 0
			s.Prune(i - 2048)
		}
		s.Set(i, 10)
	}
}

func BenchmarkTimeGet(b *testing.B) {
	var s timeSet
	for i := binmap.Bin(0); i < 1<<16; i += 2 {
		s.Set(i, 10)
	}
	b.ResetTimer()

	const m = (1 << 16) - 1
	var n int64
	for i := binmap.Bin(0); i < binmap.Bin(b.N*2); i += 2 {
		nn, _ := s.Get(i & m)
		n += nn
	}
}

func BenchmarkMapThingPrune(b *testing.B) {
	s := map[binmap.Bin]int64{}

	var n int
	for i := binmap.Bin(0); i < binmap.Bin(b.N*2); i += 2 {
		if n++; n >= 1024 && i >= 4096 {
			n = 0
			min := i - 4096
			max := i - 2048
			for j := min; j <= max; j += 2 {
				delete(s, j)
			}
		}
		s[i] = 10
	}
}

func BenchmarkMapThing(b *testing.B) {
	s := map[binmap.Bin]int64{}
	for i := binmap.Bin(0); i < 1<<16; i += 2 {
		s[i] = 10
	}
	b.ResetTimer()

	const m = (1 << 16) - 1
	var n int64
	for i := binmap.Bin(0); i < binmap.Bin(b.N*2); i += 2 {
		nn := s[i&m]
		n += nn
	}
}
