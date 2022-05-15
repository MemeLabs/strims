// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mathutil

import "golang.org/x/exp/constraints"

func Min[T constraints.Ordered](ns ...T) (n T) {
	if len(ns) == 0 {
		return
	}

	n = ns[0]
	for i := 1; i < len(ns); i++ {
		if ns[i] < n {
			n = ns[i]
		}
	}
	return
}

func Max[T constraints.Ordered](ns ...T) (n T) {
	if len(ns) == 0 {
		return
	}

	n = ns[0]
	for i := 1; i < len(ns); i++ {
		if ns[i] > n {
			n = ns[i]
		}
	}
	return
}

func Clamp[T constraints.Ordered](v, min, max T) T {
	if v < min {
		return min
	} else if v > max {
		return max
	}
	return v
}
