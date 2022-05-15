// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpc

import (
	"encoding/binary"

	"lukechampine.com/uint128"
)

// Block512 ...
type Block512 [64]byte

// Block 128 bit chunk
type Block [16]byte

func blockFromUint(i uint64) (res Block) {
	binary.LittleEndian.PutUint64(res[:], i)
	return
}

func blockFromBytes(v []byte) (b Block) {
	copy(b[:], v)
	return
}

func clmulBlock(a, b Block) (r [2]Block) {
	a0 := uint128.From64(binary.LittleEndian.Uint64(a[:8]))
	a1 := uint128.From64(binary.LittleEndian.Uint64(a[8:]))
	b0 := uint128.From64(binary.LittleEndian.Uint64(b[:8]))
	b1 := uint128.From64(binary.LittleEndian.Uint64(b[8:]))

	zero := clmulUint128(a0, b0)
	one := clmulUint128(a0, b1)
	two := clmulUint128(a1, b0)
	three := clmulUint128(a1, b1)

	tmp := one.Xor(two)
	ll := tmp.Lsh(64)
	rl := tmp.Rsh(64)

	x := zero.Xor(ll)
	y := three.Xor(rl)

	x.PutBytes(r[0][:])
	y.PutBytes(r[1][:])
	return
}

// clmulUint128 compute the product of a and b using carry-less multiplication
func clmulUint128(a, b uint128.Uint128) (r uint128.Uint128) {
	for !a.Equals64(0) {
		r = r.Xor(a.And64(1).Mul(b))
		a = a.Rsh(1)
		b = b.Lsh(1)
	}
	return r
}
