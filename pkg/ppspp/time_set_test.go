package ppspp

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/stretchr/testify/assert"
)

func assertTimeSetGetEqual(t *testing.T, s *timeSet, b binmap.Bin, expectedV int64, expectedOK bool) {
	actualV, actualOK := s.Get(b)
	assert.Equal(t, expectedV, actualV, "value mismatch at bin %d", b)
	assert.Equal(t, expectedOK, actualOK, "ok mismatch at bin %d", b)
}

func TestTimeSetGet(t *testing.T) {
	s := &timeSet{}
	s.Set(1, 100)
	s.Set(5, 200)
	s.Set(7, 300)
	s.Set(15, 400)

	cases := []struct {
		bin   binmap.Bin
		value int64
		ok    bool
	}{
		{0, 100, true},
		{4, 200, true},
		{5, 200, true},
		{7, 300, true},
		{9, 300, true},
		{15, 400, true},
		{19, 400, true},
		{31, 0, false},
		{32, 0, false},
	}
	for _, c := range cases {
		assertTimeSetGetEqual(t, s, c.bin, c.value, c.ok)
	}
}

func TestTimeSetUnset(t *testing.T) {
	s := &timeSet{}
	s.Set(3, 100)
	s.Set(9, 200)
	s.Set(23, 300)

	s.Unset(9)
	assertTimeSetGetEqual(t, s, 2, 100, true)
	assertTimeSetGetEqual(t, s, 10, 0, false)
	assertTimeSetGetEqual(t, s, 25, 300, true)

	s.Set(11, 400)
	assertTimeSetGetEqual(t, s, 2, 100, true)
	assertTimeSetGetEqual(t, s, 10, 400, true)
	assertTimeSetGetEqual(t, s, 25, 300, true)

	s.Unset(7)
	assertTimeSetGetEqual(t, s, 2, 0, false)
	assertTimeSetGetEqual(t, s, 10, 0, false)
	assertTimeSetGetEqual(t, s, 25, 300, true)
}

func TestTimeSetPrune(t *testing.T) {
	s := &timeSet{}
	s.Set(3, 100)
	s.Set(9, 200)
	s.Set(23, 300)

	s.Prune(16)
	assertTimeSetGetEqual(t, s, 2, 0, false)
	assertTimeSetGetEqual(t, s, 10, 0, false)
	assertTimeSetGetEqual(t, s, 25, 300, true)
}

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
