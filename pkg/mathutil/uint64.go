package mathutil

func MaxUint64(ns ...uint64) uint64 {
	var n uint64
	for i := range ns {
		if ns[i] > n {
			n = ns[i]
		}
	}
	return n
}
