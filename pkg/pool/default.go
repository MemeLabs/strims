// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package pool

import "math"

// DefaultPool ...
var DefaultPool = New(10)

// MaxSize ...
func MaxSize() int {
	return math.MaxUint16
}

// Get ...
func Get(size int) *[]byte {
	return DefaultPool.Get(size)
}

// Put ...
func Put(b *[]byte) {
	DefaultPool.Put(b)
}
