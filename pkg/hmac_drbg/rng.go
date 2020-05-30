package hmac_drbg

import (
	"bufio"
	"crypto/rand"
	"hash"
	"io"
	"math/big"

	"github.com/minio/blake2b-simd"
)

// NewRNG ...
func NewRNG(h func() hash.Hash, seed []byte) *RNG {
	r := NewReader(h, seed)
	return &RNG{
		r: bufio.NewReaderSize(r, r.Size()),
	}
}

// NewDefaultRNG ...
func NewDefaultRNG() *RNG {
	seed := make([]byte, 32)
	if _, err := rand.Read(seed); err != nil {
		panic(err)
	}

	return NewRNG(blake2b.New512, seed)
}

// RNG ...
type RNG struct {
	r *bufio.Reader
}

// Uint32 ...
func (a *RNG) Uint32() (n uint32) {
	for i := 0; i < 4; i++ {
		b, err := a.r.ReadByte()
		if err != nil {
			panic(err)
		}
		n |= uint32(b) << (i * 8)
	}
	return
}

// Uint64 ...
func (a *RNG) Uint64() (n uint64) {
	for i := 0; i < 8; i++ {
		b, err := a.r.ReadByte()
		if err != nil {
			panic(err)
		}
		n |= uint64(b) << (i * 8)
	}
	return
}

// BigInt ...
func (a *RNG) BigInt(size int) *big.Int {
	b := make([]byte, size)
	if _, err := a.Read(b); err != nil {
		panic(err)
	}
	return big.NewInt(0).SetBytes(b)
}

// Read fills b with random bytes
func (a *RNG) Read(b []byte) (int, error) {
	return io.ReadFull(a.r, b)
}
