// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chanutil

func AppendAll[T any](s []T, ch <-chan T) []T {
	if cap(s)-len(s) < len(ch) {
		t := s
		s = make([]T, len(s), len(s)+len(ch))
		copy(s, t)
	}

	for {
		select {
		case v, ok := <-ch:
			if !ok {
				return s
			}
			s = append(s, v)
		default:
			return s
		}
	}
}
