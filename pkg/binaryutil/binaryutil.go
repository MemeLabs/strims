// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package binaryutil

import "math/bits"

var lens = [65]byte{
	1, 1, 1, 1, 1, 1, 1, 1,
	2, 2, 2, 2, 2, 2, 2,
	3, 3, 3, 3, 3, 3, 3,
	4, 4, 4, 4, 4, 4, 4,
	5, 5, 5, 5, 5, 5, 5,
	6, 6, 6, 6, 6, 6, 6,
	7, 7, 7, 7, 7, 7, 7,
	8, 8, 8, 8, 8, 8, 8,
	9, 9, 9, 9, 9, 9, 9,
	10,
}

func UvarintLen(v uint64) int {
	return int(lens[bits.Len64(v)])
}

func VarintLen(v int64) int {
	uv := uint64(v) << 1
	if v < 0 {
		uv = ^uv
	}
	return UvarintLen(uv)
}
