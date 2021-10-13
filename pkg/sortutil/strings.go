package sortutil

import "strings"

func EqualStrings(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := 0; i < len(a); i++ {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// DiffStrings ...
func DiffStrings(prev, next []string) (removed, added []string) {
	for i, j := 0, 0; i < len(prev) || j < len(next); {
		var d int
		if i == len(prev) {
			d = 1
		} else if j == len(next) {
			d = -1
		} else {
			d = strings.Compare(prev[i], next[j])
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
