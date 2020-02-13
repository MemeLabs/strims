package mpc

import (
	"testing"
)

func TestAESRNG(t *testing.T) {
	seed := [16]byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}
	rng, err := NewAESRNG(seed[:])
	if err != nil {
		t.Error(err)
		t.FailNow()
	}

	var v [16]byte
	rng.Read(v[:])

	r := [16]byte{181, 112, 57, 74, 222, 107, 231, 247, 116, 218, 250, 128, 240, 47, 242, 146}
	if v != r {
		t.Errorf("expected %x received %x", r, v)
		t.FailNow()
	}
}

func BenchmarkAESRNG(b *testing.B) {
	seed := [16]byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}
	rng, err := NewAESRNG(seed[:])
	if err != nil {
		panic(err)
	}

	b.ResetTimer()

	r := make([]byte, 16)
	for i := 0; i < b.N; i++ {
		rng.Read(r)
	}
}
