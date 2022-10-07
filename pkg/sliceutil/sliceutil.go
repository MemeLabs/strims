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

func Find[T any](vs []T, fn func(v T) bool) (v T, ok bool) {
	for _, v := range vs {
		if fn(v) {
			return v, true
		}
	}
	return
}

func Includes[T any](vs []T, fn func(v T) bool) bool {
	_, ok := Find(vs, fn)
	return ok
}

func Filter[T any](vs []T, fn func(i int) bool) []T {
	c := make([]T, 0, len(vs))
	for i, v := range vs {
		if fn(i) {
			c = append(c, v)
		}
	}
	return c
}
