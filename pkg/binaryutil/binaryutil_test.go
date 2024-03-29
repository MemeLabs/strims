// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package binaryutil

import (
	"encoding/binary"
	"math"
	"testing"

	"github.com/stretchr/testify/assert"
)

var BenchmarkUvarintLenResult int

func BenchmarkUvarintLen(b *testing.B) {
	numbers := make([]uint64, 65)
	for i := 0; i <= 64; i++ {
		numbers[i] = uint64(math.MaxUint64 >> i)
	}

	b.ResetTimer()

	var r int
	for i := 0; i < b.N; i++ {
		r += UvarintLen(numbers[i&64])
	}
	BenchmarkUvarintLenResult = r
}

func TestUvarintLen(t *testing.T) {
	b := make([]byte, binary.MaxVarintLen64)
	for i := 0; i <= 64; i++ {
		v := uint64(math.MaxUint64 >> i)
		assert.Equal(t, binary.PutUvarint(b, v), UvarintLen(v))
	}
}

func TestVarintLen(t *testing.T) {
	b := make([]byte, binary.MaxVarintLen64)
	for i := 0; i <= 63; i++ {
		v := int64(math.MaxInt64 >> i)
		assert.Equal(t, binary.PutVarint(b, v), VarintLen(v))
		assert.Equal(t, binary.PutVarint(b, -v), VarintLen(-v))
	}
}
