package encoding

func maxInt(n int, ns ...int) int {
	for _, v := range ns {
		if v > n {
			n = v
		}
	}
	return n
}

func minInt(n int, ns ...int) int {
	for _, v := range ns {
		if v < n {
			n = v
		}
	}
	return n
}
