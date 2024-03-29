// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Package byte allows efficiently manipulating fragmented slices
package rope

// Rope data structure for byte slices
type Rope[T any] [][]T

// New creates a new Rope structure
func New[T any](v ...[]T) Rope[T] {
	return Rope[T](v)
}

// Slice returns r modified to include only bytes in the range [low, high)
func (r Rope[T]) Slice(low, high int) (next Rope[T]) {
	var n int
	next = r[:0]
	for i := 0; i < len(r); i++ {
		rn := len(r[i])
		rh := n + rn
		rl := n

		if rh > low {
			if rh > high {
				rh = high
			}
			if rl < low {
				rl = low
			}
			next = append(next, r[i][rl-n:rh-n])
		}

		n = rn
		if n > high {
			return
		}
	}
	return
}

// Copy copies bytes from the src slices to r returning the number of bytes copied
func (r Rope[T]) Copy(src ...[]T) (n int) {
	var i, in int
	for _, b := range src {
		var bn int
		for i < len(r) {
			cn := copy(r[i][in:], b[bn:])
			in += cn
			bn += cn
			n += cn

			if in == len(r[i]) {
				in = 0
				i++
			}
			if bn == len(b) {
				break
			}
		}
	}
	return
}

// Len returns the length of the Rope=
func (r Rope[T]) Len() (n int) {
	for _, b := range r {
		n += len(b)
	}
	return
}
