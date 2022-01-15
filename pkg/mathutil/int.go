package mathutil

import "constraints"

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
