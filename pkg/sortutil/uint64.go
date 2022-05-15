// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package sortutil

import (
	"sort"
)

// Uint64 ...
func Uint64(a []uint64) {
	sort.Sort(Uint64Slice(a))
}

// SearchUint64 ...
func SearchUint64(a []uint64, x uint64) int {
	return sort.Search(len(a), func(i int) bool {
		return a[i] < x
	})
}

// DiffUint64 ...
func DiffUint64(prev, next []uint64) (removed, added []uint64) {
	for i, j := 0, 0; i < len(prev) || j < len(next); {
		var d int
		if i == len(prev) {
			d = 1
		} else if j == len(next) {
			d = -1
		} else if prev[i] < next[j] {
			d = -1
		} else if prev[i] > next[j] {
			d = 1
		}

		switch {
		case d < 0:
			removed = append(removed, prev[i])
			i++
		case d == 0:
			i++
			j++
		case d > 0:
			added = append(added, next[j])
			j++
		}
	}
	return
}

// Uint64Slice ...
type Uint64Slice []uint64

func (a Uint64Slice) Len() int           { return len(a) }
func (a Uint64Slice) Swap(i, j int)      { a[i], a[j] = a[j], a[i] }
func (a Uint64Slice) Less(i, j int) bool { return a[i] < a[j] }
