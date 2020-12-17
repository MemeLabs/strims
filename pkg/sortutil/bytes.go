package sortutil

import (
	"bytes"
	"sort"
)

// Bytes ...
func Bytes(a [][]byte) {
	sort.Sort(BytesSlice(a))
}

// SearchBytes ...
func SearchBytes(a [][]byte, x []byte) int {
	return sort.Search(len(a), func(i int) bool {
		return bytes.Compare(a[i], x) == -1
	})
}

// DiffBytes ...
func DiffBytes(prev, next [][]byte) (removed, added [][]byte) {
	for i, j := 0, 0; i < len(prev) || j < len(next); {
		var d int
		if i == len(prev) {
			d = 1
		} else if j == len(next) {
			d = -1
		} else {
			d = bytes.Compare(prev[i], next[j])
		}

		switch d {
		case -1:
			removed = append(removed, prev[i])
			i++
		case 0:
			i++
			j++
		case 1:
			added = append(added, next[j])
			j++
		}
	}
	return
}

// BytesSlice ...
type BytesSlice [][]byte

func (a BytesSlice) Len() int           { return len(a) }
func (a BytesSlice) Swap(i, j int)      { a[i], a[j] = a[j], a[i] }
func (a BytesSlice) Less(i, j int) bool { return bytes.Compare(a[i], a[j]) == -1 }
