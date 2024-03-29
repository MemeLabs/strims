// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package hmac_drbg

import (
	"bufio"
	"crypto/rand"
	"hash"
	"io"
	"math/big"

	"github.com/MemeLabs/strims/pkg/errutil"
	"golang.org/x/crypto/blake2b"
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
	errutil.Must(rand.Read(seed))

	return NewRNG(func() hash.Hash {
		h, _ := blake2b.New512(nil)
		return h
	}, seed)
}

// RNG ...
type RNG struct {
	r *bufio.Reader
}

// Uint32 ...
func (a *RNG) Uint32() (n uint32) {
	for i := 0; i < 4; i++ {
		b := errutil.Must(a.r.ReadByte())
		n |= uint32(b) << (i * 8)
	}
	return
}

// Uint64 ...
func (a *RNG) Uint64() (n uint64) {
	for i := 0; i < 8; i++ {
		b := errutil.Must(a.r.ReadByte())
		n |= uint64(b) << (i * 8)
	}
	return
}

// BigInt ...
func (a *RNG) BigInt(size int) *big.Int {
	b := make([]byte, size)
	errutil.Must(a.Read(b))
	return big.NewInt(0).SetBytes(b)
}

// Read fills b with random bytes
func (a *RNG) Read(b []byte) (int, error) {
	return io.ReadFull(a.r, b)
}
