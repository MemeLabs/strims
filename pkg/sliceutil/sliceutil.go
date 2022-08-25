// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package sliceutil

func Remove[T comparable](vs []T, v T) []T {
	for i := range vs {
		if vs[i] == v {
			return RemoveAt(vs, i)
		}
	}
	return vs
}

func RemoveAt[T comparable](vs []T, i int) []T {
	l := len(vs) - 1
	vs[i] = vs[l]
	var empty T
	vs[l] = empty
	return vs[:l]
}
